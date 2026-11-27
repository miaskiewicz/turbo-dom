//! Transient native layout for already-computed, renderer-admitted styles.
//! This owns no DOM, selector state, cache or JS references. A world stamp is
//! echoed as provenance only; the caller must still validate its complete world.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use turbo_dom_layout::layout::value::Edges;
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_styled_tree_with_native_text, GeometryKind, LayoutSnapshot, NativeTextOptions,
    Rect, SnapshotInputs, SourceId, SourceIdentity, SourceKind, SourceTextUnits, UsedBoxMetrics,
};
#[cfg(not(target_arch = "wasm32"))]
use turbo_dom_layout::text::SystemFontPolicy;
use turbo_dom_layout::{
    Attr, ComputedStyle, Diagnostics, FontRegistry, ImageCtx, StyledElement, StyledNode,
    StyledText, Tag,
};

const HTML: &str = "http://www.w3.org/1999/xhtml";
const SVG: &str = "http://www.w3.org/2000/svg";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneRequest {
    pub world: Vec<u16>,
    pub viewport: SceneViewport,
    pub fonts: SceneFonts,
    pub limits: SceneLimits,
    pub nodes: Vec<SceneNode>,
    pub requested: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneViewport {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneFonts {
    NativeHtmlSystem,
    BundledHtml,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneLimits {
    #[serde(rename = "maxNodes")]
    pub nodes: usize,
    #[serde(rename = "maxDepth")]
    pub depth: usize,
    #[serde(rename = "maxPathUnits")]
    pub path_units: usize,
    #[serde(rename = "maxTextUnits")]
    pub text_units: usize,
    #[serde(rename = "maxAttributes")]
    pub attributes: usize,
    #[serde(rename = "maxAttributeUnits")]
    pub attribute_units: usize,
    #[serde(rename = "maxProperties")]
    pub properties: usize,
    #[serde(rename = "maxStyleUnits")]
    pub style_units: usize,
    #[serde(rename = "maxWorldUnits")]
    pub world_units: usize,
    #[serde(rename = "maxRequests")]
    pub requests: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub node: SceneNodeKind,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneNodeKind {
    Element {
        #[serde(rename = "tagUnits")]
        tag_units: Vec<u16>,
        #[serde(rename = "namespaceUnits")]
        namespace_units: Vec<u16>,
        attrs: Vec<SceneAttribute>,
        computed: Vec<SceneProperty>,
    },
    Text {
        #[serde(rename = "textUnits")]
        text_units: Vec<u16>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneAttribute {
    pub name_units: Vec<u16>,
    #[serde(rename = "namespaceUnits")]
    pub namespace: Option<Vec<u16>>,
    pub value_units: Vec<u16>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneProperty {
    pub name_units: Vec<u16>,
    pub value_units: Vec<u16>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneResponse {
    pub world: Vec<u16>,
    pub fonts: SceneFonts,
    pub measurements: Vec<SceneMeasurement>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SceneMeasurement {
    pub id: u32,
    pub result: SceneResult,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SceneResult {
    UsedBox {
        geometry: SceneBox,
    },
    Unavailable {
        reason: SceneUnavailable,
        detail: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneUnavailable {
    NotElement,
    Excluded,
    SvgDescendant,
    InlineGeometry,
    MissingBox,
    FragmentedBox,
    InlineBox,
    NonFiniteBox,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneBox {
    pub border_box: SceneRect,
    pub content_box: SceneRect,
    pub padding: SceneEdges,
    pub border: SceneEdges,
    // The public native metrics do not certify distributed auto margins.
    pub margin: Option<SceneEdges>,
    pub margin_unavailable: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SceneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct SceneEdges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneError {
    Budget {
        limit: &'static str,
    },
    Invalid {
        id: Option<u32>,
        reason: &'static str,
    },
    Unsupported {
        id: Option<u32>,
        reason: &'static str,
    },
    Layout {
        message: String,
    },
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Budget { limit } => write!(f, "scene exceeds {limit}"),
            Self::Invalid { id, reason } => write!(f, "invalid scene {id:?}: {reason}"),
            Self::Unsupported { id, reason } => write!(f, "unsupported scene {id:?}: {reason}"),
            Self::Layout { message } => f.write_str(message),
        }
    }
}
impl std::error::Error for SceneError {}

type Result<T> = std::result::Result<T, SceneError>;

struct SceneGraph {
    by_id: BTreeMap<u32, usize>,
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,
    paths: Vec<Vec<usize>>,
    hidden: Vec<bool>,
}

#[derive(Default)]
struct Counts {
    text: usize,
    attributes: usize,
    attribute_units: usize,
    properties: usize,
    style_units: usize,
    paths: usize,
}

fn charge(total: &mut usize, amount: usize, limit: usize, name: &'static str) -> Result<()> {
    *total = total
        .checked_add(amount)
        .filter(|sum| *sum <= limit)
        .ok_or(SceneError::Budget { limit: name })?;
    Ok(())
}

fn checked_len(value: usize, limit: usize, name: &'static str) -> Result<()> {
    if value > limit {
        return Err(SceneError::Budget { limit: name });
    }
    Ok(())
}

fn scalar(units: &[u16], id: u32) -> Result<String> {
    String::from_utf16(units).map_err(|_| SceneError::Unsupported {
        id: Some(id),
        reason: "non-scalar tag, attribute or computed-style projection",
    })
}

fn invalid(id: u32, reason: &'static str) -> SceneError {
    SceneError::Invalid {
        id: Some(id),
        reason,
    }
}

fn validate_header(request: &SceneRequest) -> Result<()> {
    let viewport = request.viewport;
    if !viewport.width.is_finite()
        || !viewport.height.is_finite()
        || viewport.width < 0.0
        || viewport.height < 0.0
    {
        return Err(SceneError::Invalid {
            id: None,
            reason: "viewport must be finite and non-negative",
        });
    }
    checked_len(request.nodes.len(), request.limits.nodes, "node limit")?;
    checked_len(
        request.world.len(),
        request.limits.world_units,
        "world-unit limit",
    )?;
    checked_len(
        request.requested.len(),
        request.limits.requests,
        "request limit",
    )
}

fn count_attribute(
    attribute: &SceneAttribute,
    counts: &mut Counts,
    limits: &SceneLimits,
) -> Result<()> {
    charge(
        &mut counts.attributes,
        1,
        limits.attributes,
        "attribute limit",
    )?;
    for units in [&attribute.name_units, &attribute.value_units] {
        charge(
            &mut counts.attribute_units,
            units.len(),
            limits.attribute_units,
            "attribute-unit limit",
        )?;
    }
    if let Some(units) = &attribute.namespace {
        charge(
            &mut counts.attribute_units,
            units.len(),
            limits.attribute_units,
            "attribute-unit limit",
        )?;
    }
    Ok(())
}

fn count_node(node: &SceneNode, counts: &mut Counts, limits: &SceneLimits) -> Result<()> {
    match &node.node {
        SceneNodeKind::Text { text_units } => charge(
            &mut counts.text,
            text_units.len(),
            limits.text_units,
            "text-unit limit",
        ),
        SceneNodeKind::Element {
            tag_units,
            namespace_units,
            attrs,
            computed,
        } => {
            for units in [tag_units, namespace_units] {
                charge(
                    &mut counts.attribute_units,
                    units.len(),
                    limits.attribute_units,
                    "attribute-unit limit",
                )?;
            }
            for attr in attrs {
                count_attribute(attr, counts, limits)?;
            }
            count_properties(computed, counts, limits)?;
            Ok(())
        }
    }
}

fn count_properties(
    properties: &[SceneProperty],
    counts: &mut Counts,
    limits: &SceneLimits,
) -> Result<()> {
    charge(
        &mut counts.properties,
        properties.len(),
        limits.properties,
        "property limit",
    )?;
    for property in properties {
        for units in [&property.name_units, &property.value_units] {
            charge(
                &mut counts.style_units,
                units.len(),
                limits.style_units,
                "style-unit limit",
            )?;
        }
    }
    Ok(())
}

fn ascii_eq(units: &[u16], ascii: &[u8]) -> bool {
    units.len() == ascii.len()
        && units
            .iter()
            .zip(ascii)
            .all(|(unit, byte)| *unit == u16::from(*byte))
}

fn is_hidden(node: &SceneNode) -> bool {
    let SceneNodeKind::Element { computed, .. } = &node.node else {
        return false;
    };
    computed.iter().any(|property| {
        ascii_eq(&property.name_units, b"display") && ascii_eq(&property.value_units, b"none")
    })
}

fn scene_graph(request: &SceneRequest) -> Result<SceneGraph> {
    let mut graph = SceneGraph {
        by_id: BTreeMap::new(),
        children: vec![Vec::new(); request.nodes.len()],
        roots: Vec::new(),
        paths: Vec::new(),
        hidden: Vec::new(),
    };
    let mut counts = Counts::default();
    for (index, node) in request.nodes.iter().enumerate() {
        if graph.by_id.contains_key(&node.id) {
            return Err(invalid(node.id, "duplicate source ID"));
        }
        count_node(node, &mut counts, &request.limits)?;
        let (path, parent_hidden) = node_path(request, &mut graph, index, &mut counts)?;
        graph.paths.push(path);
        graph.hidden.push(parent_hidden || is_hidden(node));
        graph.by_id.insert(node.id, index);
    }
    validate_requested(request, &graph)?;
    Ok(graph)
}

fn node_path(
    request: &SceneRequest,
    graph: &mut SceneGraph,
    index: usize,
    counts: &mut Counts,
) -> Result<(Vec<usize>, bool)> {
    let node = &request.nodes[index];
    if let Some(parent) = node.parent {
        let parent = *graph
            .by_id
            .get(&parent)
            .ok_or(invalid(node.id, "parent must precede child"))?;
        if !matches!(request.nodes[parent].node, SceneNodeKind::Element { .. }) {
            return Err(invalid(node.id, "text nodes cannot own children"));
        }
        let depth = graph.paths[parent]
            .len()
            .checked_add(1)
            .ok_or(SceneError::Budget {
                limit: "depth limit",
            })?;
        check_path_budget(depth, counts, &request.limits)?;
        let mut path = graph.paths[parent].clone();
        path.push(graph.children[parent].len());
        graph.children[parent].push(index);
        Ok((path, graph.hidden[parent]))
    } else {
        check_path_budget(1, counts, &request.limits)?;
        let path = vec![graph.roots.len()];
        graph.roots.push(index);
        Ok((path, false))
    }
}

fn check_path_budget(depth: usize, counts: &mut Counts, limits: &SceneLimits) -> Result<()> {
    checked_len(depth, limits.depth, "depth limit")?;
    charge(
        &mut counts.paths,
        depth,
        limits.path_units,
        "path-unit limit",
    )
}

fn validate_requested(request: &SceneRequest, graph: &SceneGraph) -> Result<()> {
    let mut seen = BTreeSet::new();
    for id in &request.requested {
        if !graph.by_id.contains_key(id) {
            return Err(invalid(*id, "requested source is absent"));
        }
        if !seen.insert(*id) {
            return Err(invalid(*id, "requested source is duplicated"));
        }
    }
    Ok(())
}

struct StyledForest<'a> {
    nodes: Vec<StyledNode>,
    identities: Vec<SourceIdentity>,
    text: Vec<SourceTextUnits<'a>>,
    svg_viewports: Vec<SourceId>,
    auto_margin: BTreeSet<u32>,
}

fn computed_style(id: u32, properties: &[SceneProperty]) -> Result<ComputedStyle> {
    let mut pairs = BTreeMap::new();
    for property in properties {
        let name = scalar(&property.name_units, id)?;
        let value = scalar(&property.value_units, id)?;
        if pairs.insert(name, value).is_some() {
            return Err(invalid(id, "computed properties must be unique"));
        }
    }
    let style = ComputedStyle::from_pairs(pairs);
    if style
        .get("zoom")
        .is_some_and(|value| !matches!(value, "normal" | "1" | "100%"))
    {
        return Err(SceneError::Unsupported {
            id: Some(id),
            reason: "non-neutral zoom requires a canonical zoom adapter",
        });
    }
    Ok(style)
}

fn projected_attributes(id: u32, attributes: &[SceneAttribute]) -> Result<Vec<Attr>> {
    let mut result = Vec::new();
    let mut names = BTreeSet::new();
    for attribute in attributes {
        // The public Attr projection has no namespace slot. A foreign-namespace
        // width/src must never become an HTML reflected attribute.
        if attribute
            .namespace
            .as_ref()
            .is_some_and(|units| !units.is_empty())
        {
            continue;
        }
        let name = scalar(&attribute.name_units, id)?;
        if !names.insert(name.clone()) {
            return Err(invalid(id, "attributes must be unique"));
        }
        result.push(Attr {
            name,
            value: scalar(&attribute.value_units, id)?,
        });
    }
    Ok(result)
}

fn parent_namespace<'a>(
    request: &'a SceneRequest,
    graph: &SceneGraph,
    node: &SceneNode,
) -> Option<&'a [u16]> {
    let index = graph.by_id.get(&node.parent?)?;
    let SceneNodeKind::Element {
        namespace_units, ..
    } = &request.nodes[*index].node
    else {
        return None;
    };
    Some(namespace_units)
}

fn has_svg_root(request: &SceneRequest, graph: &SceneGraph, mut index: usize) -> bool {
    loop {
        let SceneNodeKind::Element {
            tag_units,
            namespace_units,
            ..
        } = &request.nodes[index].node
        else {
            return false;
        };
        if !ascii_eq(namespace_units, SVG.as_bytes()) {
            return false;
        }
        if ascii_eq(tag_units, b"svg") {
            return true;
        }
        let Some(parent) = request.nodes[index].parent else {
            return false;
        };
        index = graph.by_id[&parent];
    }
}

fn admit_element(
    request: &SceneRequest,
    graph: &SceneGraph,
    index: usize,
    tag: &str,
    namespace: &[u16],
) -> Result<bool> {
    let node = &request.nodes[index];
    let hidden = graph.hidden[index];
    if ascii_eq(namespace, HTML.as_bytes()) {
        return admit_html_element(request, graph, index, tag);
    }
    if !ascii_eq(namespace, SVG.as_bytes()) {
        return Err(SceneError::Unsupported {
            id: Some(node.id),
            reason: "native scene namespace is unsupported",
        });
    }
    if !hidden && (tag == "foreignObject" || !has_svg_root(request, graph, index)) {
        return Err(SceneError::Unsupported {
            id: Some(node.id),
            reason: "SVG requires a proven viewport and no foreignObject layout",
        });
    }
    let parent_svg = parent_namespace(request, graph, node)
        .is_some_and(|namespace| ascii_eq(namespace, SVG.as_bytes()));
    Ok(!hidden && tag == "svg" && !parent_svg)
}

fn admit_html_element(
    request: &SceneRequest,
    graph: &SceneGraph,
    index: usize,
    tag: &str,
) -> Result<bool> {
    let node = &request.nodes[index];
    if graph.hidden[index] {
        return Ok(false);
    }
    if tag == "svg" || tag.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(SceneError::Unsupported {
            id: Some(node.id),
            reason: "HTML tag cannot be projected as an SVG or case-normalized native tag",
        });
    }
    if parent_namespace(request, graph, node)
        .is_some_and(|namespace| ascii_eq(namespace, SVG.as_bytes()))
    {
        return Err(SceneError::Unsupported {
            id: Some(node.id),
            reason: "HTML inside SVG requires a foreignObject integration context",
        });
    }
    Ok(false)
}

fn admit_html_resources(id: u32, tag: &str, attributes: &[Attr]) -> Result<()> {
    if [
        "img", "iframe", "frame", "object", "embed", "video", "audio", "canvas",
    ]
    .contains(&tag)
    {
        return Err(SceneError::Unsupported {
            id: Some(id),
            reason: "intrinsic/frame resource state is not supplied by this scene protocol",
        });
    }
    let image_input = tag == "input"
        && attributes.iter().any(|attr| {
            attr.name.eq_ignore_ascii_case("type") && attr.value.eq_ignore_ascii_case("image")
        });
    if image_input {
        return Err(SceneError::Unsupported {
            id: Some(id),
            reason: "image input needs actual intrinsic resources",
        });
    }
    Ok(())
}

fn auto_margin(style: &ComputedStyle) -> bool {
    style.iter().any(|(name, value)| {
        (name == "margin" || name.starts_with("margin-"))
            && value.split_ascii_whitespace().any(|word| word == "auto")
    })
}

fn project_node<'a>(
    request: &'a SceneRequest,
    graph: &SceneGraph,
    index: usize,
    forest: &mut StyledForest<'a>,
) -> Result<StyledNode> {
    let node = &request.nodes[index];
    match &node.node {
        SceneNodeKind::Text { text_units } => {
            forest.text.push(SourceTextUnits {
                source_id: SourceId(u64::from(node.id)),
                units: text_units,
            });
            Ok(StyledNode::Text(StyledText {
                text: String::from_utf16_lossy(text_units),
                source: None,
            }))
        }
        SceneNodeKind::Element {
            tag_units,
            namespace_units,
            attrs,
            computed,
        } => project_element(
            request,
            graph,
            index,
            forest,
            ElementInput {
                tag_units,
                namespace_units,
                attrs,
                computed,
            },
        ),
    }
}

struct ElementInput<'a> {
    tag_units: &'a [u16],
    namespace_units: &'a [u16],
    attrs: &'a [SceneAttribute],
    computed: &'a [SceneProperty],
}

fn project_element<'a>(
    request: &'a SceneRequest,
    graph: &SceneGraph,
    index: usize,
    forest: &mut StyledForest<'a>,
    input: ElementInput<'_>,
) -> Result<StyledNode> {
    let ElementInput {
        tag_units,
        namespace_units,
        attrs,
        computed,
    } = input;
    let node = &request.nodes[index];
    let tag = project_tag(node.id, tag_units)?;
    let viewport = admit_element(request, graph, index, &tag, namespace_units)?;
    let attrs = projected_attributes(node.id, attrs)?;
    admit_resources(graph.hidden[index], namespace_units, node.id, &tag, &attrs)?;
    let style = computed_style(node.id, computed)?;
    record_element(forest, node.id, viewport, &style);
    Ok(StyledNode::Element(StyledElement {
        tag: Tag::Html(tag),
        attrs,
        style,
        children: Vec::new(),
        source: None,
    }))
}

fn project_tag(id: u32, units: &[u16]) -> Result<String> {
    let tag = scalar(units, id)?;
    if tag.is_empty() {
        return Err(invalid(id, "element tag is empty"));
    }
    Ok(tag)
}

fn admit_resources(
    hidden: bool,
    namespace: &[u16],
    id: u32,
    tag: &str,
    attrs: &[Attr],
) -> Result<()> {
    if !hidden && ascii_eq(namespace, HTML.as_bytes()) {
        admit_html_resources(id, tag, attrs)?;
    }
    Ok(())
}

fn record_element(forest: &mut StyledForest<'_>, id: u32, viewport: bool, style: &ComputedStyle) {
    if auto_margin(style) {
        forest.auto_margin.insert(id);
    }
    if viewport {
        forest.svg_viewports.push(SourceId(u64::from(id)));
    }
}

fn styled_forest<'a>(request: &'a SceneRequest, graph: &SceneGraph) -> Result<StyledForest<'a>> {
    let mut forest = StyledForest {
        nodes: Vec::new(),
        identities: Vec::new(),
        text: Vec::new(),
        svg_viewports: Vec::new(),
        auto_margin: BTreeSet::new(),
    };
    let mut projected = Vec::with_capacity(request.nodes.len());
    for (index, node) in request.nodes.iter().enumerate() {
        forest.identities.push(SourceIdentity {
            id: SourceId(u64::from(node.id)),
            path: graph.paths[index].clone(),
        });
        projected.push(Some(project_node(request, graph, index, &mut forest)?));
    }
    for index in (0..request.nodes.len()).rev() {
        let children = graph.children[index]
            .iter()
            .map(|child| {
                projected[*child]
                    .take()
                    .expect("each child has one preceding parent")
            })
            .collect();
        if let Some(StyledNode::Element(element)) = &mut projected[index] {
            element.children = children;
        }
    }
    forest.nodes = graph
        .roots
        .iter()
        .map(|root| {
            projected[*root]
                .take()
                .expect("roots are never child entries")
        })
        .collect();
    Ok(forest)
}

fn scene_fonts(profile: SceneFonts) -> Result<FontRegistry> {
    let fonts = match profile {
        SceneFonts::BundledHtml => FontRegistry::new(),
        SceneFonts::NativeHtmlSystem => {
            #[cfg(target_arch = "wasm32")]
            {
                return Err(SceneError::Unsupported {
                    id: None,
                    reason: "system HTML fonts are unavailable in this WASM profile",
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                FontRegistry::system(SystemFontPolicy::Html).clone()
            }
        }
    };
    if fonts.is_empty() {
        return Err(SceneError::Unsupported {
            id: None,
            reason: "the selected font profile has no canonical faces",
        });
    }
    Ok(fonts)
}

#[derive(Default)]
struct MissingImages(std::cell::Cell<bool>);
impl turbo_dom_layout::ImageResolver for MissingImages {
    fn resolve(&self, _name: &str) -> Option<&[u8]> {
        self.0.set(true);
        None
    }
}

fn capture(forest: &StyledForest<'_>, request: &SceneRequest) -> Result<LayoutSnapshot> {
    let fonts = scene_fonts(request.fonts)?;
    let missing = MissingImages::default();
    let images = ImageCtx {
        resolver: &missing,
        body_height: None,
    };
    let inputs = SnapshotInputs {
        width: request.viewport.width,
        height: request.viewport.height,
        fonts: &fonts,
        images: &images,
    };
    let options = NativeTextOptions {
        original_text: &forest.text,
        svg_viewports: &forest.svg_viewports,
        layout_scale: 1.0,
    };
    let mut diagnostics = Diagnostics::default();
    let snapshot = snapshot_html_styled_tree_with_native_text(
        &forest.nodes,
        &forest.identities,
        &options,
        &inputs,
        &mut diagnostics,
    )
    .map_err(|error| SceneError::Layout {
        message: error.to_string(),
    })?;
    if missing.0.get() {
        return Err(SceneError::Unsupported {
            id: None,
            reason: "native layout requested unavailable intrinsic image metrics",
        });
    }
    if !diagnostics.is_empty() {
        return Err(SceneError::Layout {
            message: diagnostics
                .lints
                .iter()
                .map(|lint| lint.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        });
    }
    Ok(snapshot)
}

fn unavailable(reason: SceneUnavailable) -> SceneResult {
    SceneResult::Unavailable {
        reason,
        detail: None,
    }
}

impl From<Rect> for SceneRect {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}
impl From<Edges> for SceneEdges {
    fn from(edges: Edges) -> Self {
        Self {
            top: edges.top,
            right: edges.right,
            bottom: edges.bottom,
            left: edges.left,
        }
    }
}

fn finite_box(rect: Rect, used: UsedBoxMetrics) -> bool {
    let content = used.content_rect(rect);
    let edges = [used.margin, used.border, used.padding];
    [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        content.x,
        content.y,
        content.width,
        content.height,
    ]
    .into_iter()
    .all(f32::is_finite)
        && [rect.width, rect.height, content.width, content.height]
            .into_iter()
            .all(|value| value >= 0.0)
        && edges.iter().all(|edge| {
            [edge.top, edge.right, edge.bottom, edge.left]
                .into_iter()
                .all(f32::is_finite)
        })
}

fn box_result(rect: Rect, used: UsedBoxMetrics, auto: bool) -> SceneResult {
    if !finite_box(rect, used) {
        return unavailable(SceneUnavailable::NonFiniteBox);
    }
    SceneResult::UsedBox {
        geometry: SceneBox {
            border_box: rect.into(),
            content_box: used.content_rect(rect).into(),
            padding: used.padding.into(),
            border: used.border.into(),
            margin: (!auto).then(|| used.margin.into()),
            margin_unavailable: auto.then_some("native auto-margin distribution is not captured"),
        },
    }
}

fn measurement(snapshot: &LayoutSnapshot, id: u32, auto: bool) -> SceneResult {
    let source_id = SourceId(u64::from(id));
    let Some(source) = snapshot.source(source_id) else {
        return unavailable(SceneUnavailable::MissingBox);
    };
    if source.kind != SourceKind::Element {
        return unavailable(SceneUnavailable::NotElement);
    }
    if source.excluded_from_layout {
        return unavailable(SceneUnavailable::Excluded);
    }
    if snapshot.unsupported_svg_source_geometry(source_id) {
        return unavailable(SceneUnavailable::SvgDescendant);
    }
    if let Some(error) = snapshot.unsupported_inline_geometry(source_id) {
        return SceneResult::Unavailable {
            reason: SceneUnavailable::InlineGeometry,
            detail: Some(error.to_string()),
        };
    }
    measure_source_box(snapshot, source, source_id, auto)
}

fn measure_source_box(
    snapshot: &LayoutSnapshot,
    source: &turbo_dom_layout::layout_snapshot::SnapshotSource,
    id: SourceId,
    auto: bool,
) -> SceneResult {
    let mut boxes = source
        .fragment_indices
        .iter()
        .map(|index| &snapshot.fragments()[*index])
        .filter(|fragment| {
            fragment.kind == GeometryKind::BorderBox && fragment.source_id == Some(id)
        });
    let Some(fragment) = boxes.next() else {
        return unavailable(SceneUnavailable::MissingBox);
    };
    if boxes.next().is_some() {
        return unavailable(SceneUnavailable::FragmentedBox);
    }
    let Some(used) = fragment.used_box else {
        return unavailable(SceneUnavailable::MissingBox);
    };
    if used.inline_fragment {
        return unavailable(SceneUnavailable::InlineBox);
    }
    box_result(fragment.rect, used, auto)
}

/// Styles are already computed and admitted by the canonical caller. This does
/// not parse CSS, infer UA defaults, synthesize dimensions or certify cache reuse.
pub fn measure_scene(request: &SceneRequest) -> Result<SceneResponse> {
    validate_header(request)?;
    let graph = scene_graph(request)?;
    let mut response = SceneResponse {
        world: request.world.clone(),
        fonts: request.fonts,
        measurements: Vec::new(),
    };
    if request.requested.is_empty() {
        return Ok(response);
    }
    let forest = styled_forest(request, &graph)?;
    let snapshot = capture(&forest, request)?;
    response.measurements = request
        .requested
        .iter()
        .map(|id| SceneMeasurement {
            id: *id,
            result: measurement(&snapshot, *id, forest.auto_margin.contains(id)),
        })
        .collect();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }
    fn property(name: &str, value: &str) -> SceneProperty {
        SceneProperty {
            name_units: units(name),
            value_units: units(value),
        }
    }
    fn element(id: u32, parent: Option<u32>, tag: &str, width: &str) -> SceneNode {
        SceneNode {
            id,
            parent,
            node: SceneNodeKind::Element {
                tag_units: units(tag),
                namespace_units: units(HTML),
                attrs: Vec::new(),
                computed: vec![
                    property("display", "block"),
                    property("width", width),
                    property("height", "10px"),
                    property("font-size", "20px"),
                    property("font-family", "Inter"),
                ],
            },
        }
    }
    fn text(id: u32, parent: u32, value: &[u16]) -> SceneNode {
        SceneNode {
            id,
            parent: Some(parent),
            node: SceneNodeKind::Text {
                text_units: value.to_vec(),
            },
        }
    }
    fn request(nodes: Vec<SceneNode>, requested: Vec<u32>) -> SceneRequest {
        SceneRequest {
            world: vec![0xD800, 7],
            viewport: SceneViewport {
                width: 800.0,
                height: 600.0,
            },
            fonts: SceneFonts::BundledHtml,
            limits: SceneLimits {
                nodes: 100,
                depth: 30,
                path_units: 1000,
                text_units: 1000,
                attributes: 100,
                attribute_units: 10000,
                properties: 1000,
                style_units: 10000,
                world_units: 100,
                requests: 100,
            },
            nodes,
            requested,
        }
    }
    fn computed(node: &mut SceneNode) -> &mut Vec<SceneProperty> {
        let SceneNodeKind::Element { computed, .. } = &mut node.node else {
            panic!("element fixture")
        };
        computed
    }
    fn attrs(node: &mut SceneNode) -> &mut Vec<SceneAttribute> {
        let SceneNodeKind::Element { attrs, .. } = &mut node.node else {
            panic!("element fixture")
        };
        attrs
    }
    fn used(response: &SceneResponse, id: u32) -> &SceneBox {
        let measurement = response
            .measurements
            .iter()
            .find(|value| value.id == id)
            .unwrap();
        let SceneResult::UsedBox { geometry } = &measurement.result else {
            panic!("expected a genuine box: {measurement:?}")
        };
        geometry
    }

    #[test]
    fn content_basis_comes_from_a_real_contained_layout_and_keeps_world_units() {
        let mut root = element(10, None, "div", "800px");
        computed(&mut root).push(property("container-type", "inline-size"));
        let data = request(
            vec![
                root,
                element(40, Some(10), "div", "50%"),
                element(70, Some(40), "div", "50%"),
            ],
            vec![10, 40, 70],
        );
        let result = measure_scene(&data).unwrap();
        assert_eq!(result.world, [0xD800, 7]);
        assert_eq!(used(&result, 10).content_box.width, 800.0);
        assert_eq!(used(&result, 40).content_box.width, 400.0);
        assert_eq!(used(&result, 70).content_box.width, 200.0);
    }

    #[test]
    fn an_already_computed_inherited_length_is_not_recomputed_from_authored_units() {
        // An upstream 20em computed value is already200px. The child font is20px;
        // using that font to reinterpret the authored declaration would make400px.
        let data = request(
            vec![
                element(1, None, "div", "800px"),
                element(2, Some(1), "div", "200px"),
            ],
            vec![2],
        );
        assert_eq!(
            used(&measure_scene(&data).unwrap(), 2).content_box.width,
            200.0
        );
    }

    #[test]
    fn padding_border_and_nonauto_margin_are_the_native_used_metrics() {
        let mut node = element(1, None, "div", "100px");
        for side in ["top", "right", "bottom", "left"] {
            computed(&mut node).push(property(&format!("padding-{side}"), "10px"));
            computed(&mut node).push(property(&format!("border-{side}-width"), "2px"));
            computed(&mut node).push(property(&format!("border-{side}-style"), "solid"));
            computed(&mut node).push(property(&format!("margin-{side}"), "5px"));
        }
        let result = measure_scene(&request(vec![node], vec![1])).unwrap();
        let geometry = used(&result, 1);
        assert_eq!(geometry.content_box.width, 100.0);
        assert_eq!(geometry.border_box.width, 124.0);
        assert_eq!(geometry.content_box.x - geometry.border_box.x, 12.0);
        assert_eq!(geometry.padding.left, 10.0);
        assert_eq!(geometry.border.left, 2.0);
        assert_eq!(geometry.margin.as_ref().unwrap().left, 5.0);
    }

    #[test]
    fn auto_margin_is_explicitly_unavailable_instead_of_a_synthetic_zero() {
        let mut node = element(1, None, "div", "100px");
        computed(&mut node).push(property("margin-left", "auto"));
        computed(&mut node).push(property("margin-right", "auto"));
        let result = measure_scene(&request(vec![node], vec![1])).unwrap();
        let geometry = used(&result, 1);
        assert_eq!(geometry.content_box.width, 100.0);
        assert!(geometry.margin.is_none());
        assert!(geometry.margin_unavailable.is_some());
    }

    #[test]
    fn no_metrics_are_invented_for_excluded_text_or_empty_inline_sources() {
        let mut hidden = element(1, None, "div", "100px");
        computed(&mut hidden)[0] = property("display", "none");
        let mut inline = element(3, None, "span", "auto");
        computed(&mut inline)[0] = property("display", "inline");
        let data = request(vec![hidden, text(2, 1, &[0xD800]), inline], vec![1, 2, 3]);
        let result = measure_scene(&data).unwrap();
        assert!(matches!(
            result.measurements[0].result,
            SceneResult::Unavailable {
                reason: SceneUnavailable::Excluded,
                ..
            }
        ));
        assert!(matches!(
            result.measurements[1].result,
            SceneResult::Unavailable {
                reason: SceneUnavailable::NotElement,
                ..
            }
        ));
        assert!(matches!(
            result.measurements[2].result,
            SceneResult::Unavailable { .. }
        ));
    }

    #[test]
    fn source_text_units_are_preserved_beside_the_scalar_renderer_projection() {
        let data = request(
            vec![
                element(1, None, "div", "100px"),
                text(2, 1, &[0xD800, 0x61]),
            ],
            vec![1],
        );
        let graph = scene_graph(&data).unwrap();
        let forest = styled_forest(&data, &graph).unwrap();
        assert_eq!(forest.text[0].source_id, SourceId(2));
        assert_eq!(forest.text[0].units, [0xD800, 0x61]);
        assert_eq!(forest.identities[1].path, [0, 0]);
    }

    #[test]
    fn invalid_graphs_and_requests_fail_before_layout() {
        let cases = [
            request(
                vec![
                    element(1, None, "div", "10px"),
                    element(1, None, "div", "10px"),
                ],
                vec![1],
            ),
            request(
                vec![
                    element(1, Some(2), "div", "10px"),
                    element(2, None, "div", "10px"),
                ],
                vec![1],
            ),
            request(
                vec![
                    SceneNode {
                        id: 1,
                        parent: None,
                        node: SceneNodeKind::Text {
                            text_units: Vec::new(),
                        },
                    },
                    element(2, Some(1), "div", "10px"),
                ],
                vec![2],
            ),
            request(vec![element(1, None, "div", "10px")], vec![99]),
            request(vec![element(1, None, "div", "10px")], vec![1, 1]),
        ];
        for data in cases {
            assert!(matches!(
                measure_scene(&data),
                Err(SceneError::Invalid { .. })
            ));
        }
    }

    #[test]
    fn viewport_and_declared_source_budgets_are_enforced_without_large_allocations() {
        for width in [f32::NAN, f32::INFINITY, -1.0] {
            let mut data = request(Vec::new(), Vec::new());
            data.viewport.width = width;
            assert!(matches!(
                measure_scene(&data),
                Err(SceneError::Invalid { .. })
            ));
        }
        let mut data = request(
            vec![
                element(1, None, "div", "10px"),
                element(2, Some(1), "div", "10px"),
            ],
            vec![2],
        );
        data.limits.depth = 1;
        assert!(matches!(
            measure_scene(&data),
            Err(SceneError::Budget {
                limit: "depth limit"
            })
        ));
        data.limits.depth = 30;
        data.limits.path_units = 1;
        assert!(matches!(
            measure_scene(&data),
            Err(SceneError::Budget {
                limit: "path-unit limit"
            })
        ));
        let mut count = usize::MAX;
        assert!(matches!(
            charge(&mut count, 1, usize::MAX, "checked"),
            Err(SceneError::Budget { .. })
        ));
    }

    #[test]
    fn text_attribute_style_and_world_limits_are_independent() {
        let mut node = element(1, None, "div", "10px");
        attrs(&mut node).push(SceneAttribute {
            name_units: units("title"),
            namespace: None,
            value_units: units("value"),
        });
        let original = request(vec![node, text(2, 1, &units("text"))], vec![1]);
        let mut variants = Vec::new();
        let mut data = original.clone();
        data.limits.text_units = 0;
        variants.push(data);
        let mut data = original.clone();
        data.limits.attributes = 0;
        variants.push(data);
        let mut data = original.clone();
        data.limits.attribute_units = 0;
        variants.push(data);
        let mut data = original.clone();
        data.limits.style_units = 0;
        variants.push(data);
        let mut data = original.clone();
        data.limits.properties = 0;
        variants.push(data);
        let mut data = original.clone();
        data.limits.world_units = 0;
        variants.push(data);
        let mut data = original.clone();
        data.limits.nodes = 0;
        variants.push(data);
        let mut data = original;
        data.limits.requests = 0;
        variants.push(data);
        for data in variants {
            assert!(matches!(
                measure_scene(&data),
                Err(SceneError::Budget { .. })
            ));
        }
    }

    #[test]
    fn namespaces_frames_images_zoom_and_lossy_scalar_inputs_are_not_fake_boxes() {
        let mut foreign = element(1, None, "math", "100px");
        let SceneNodeKind::Element {
            namespace_units, ..
        } = &mut foreign.node
        else {
            unreachable!()
        };
        *namespace_units = units("http://www.w3.org/1998/Math/MathML");
        let mut zoom = element(1, None, "div", "100px");
        computed(&mut zoom).push(property("zoom", "2"));
        let mut lossy = element(1, None, "div", "100px");
        computed(&mut lossy).push(SceneProperty {
            name_units: units("font-family"),
            value_units: vec![0xD800],
        });
        for node in [
            foreign,
            zoom,
            lossy,
            element(1, None, "iframe", "100px"),
            element(1, None, "img", "100px"),
        ] {
            assert!(matches!(
                measure_scene(&request(vec![node], vec![1])),
                Err(SceneError::Unsupported { .. })
            ));
        }
    }

    #[test]
    fn foreign_attribute_namespace_does_not_create_a_reflected_native_width() {
        let mut node = element(1, None, "div", "100px");
        attrs(&mut node).push(SceneAttribute {
            name_units: units("width"),
            namespace: Some(units("urn:foreign")),
            value_units: units("900"),
        });
        let data = request(vec![node], vec![1]);
        let graph = scene_graph(&data).unwrap();
        let forest = styled_forest(&data, &graph).unwrap();
        assert!(forest.nodes[0].as_element().unwrap().attrs.is_empty());
        assert_eq!(
            used(&measure_scene(&data).unwrap(), 1).content_box.width,
            100.0
        );
    }

    #[test]
    fn actual_svg_viewport_does_not_block_html_basis_but_descendant_shapes_stay_unknown() {
        let mut svg = element(2, Some(1), "svg", "24px");
        computed(&mut svg)[2] = property("height", "24px");
        let SceneNodeKind::Element {
            namespace_units, ..
        } = &mut svg.node
        else {
            unreachable!()
        };
        *namespace_units = units(SVG);
        let mut path = element(3, Some(2), "path", "auto");
        let SceneNodeKind::Element {
            namespace_units, ..
        } = &mut path.node
        else {
            unreachable!()
        };
        *namespace_units = units(SVG);
        let data = request(
            vec![element(1, None, "div", "800px"), svg, path],
            vec![1, 2, 3],
        );
        let result = measure_scene(&data).unwrap();
        assert_eq!(used(&result, 1).content_box.width, 800.0);
        assert_eq!(used(&result, 2).content_box.width, 24.0);
        assert!(matches!(
            result.measurements[2].result,
            SceneResult::Unavailable {
                reason: SceneUnavailable::SvgDescendant,
                ..
            }
        ));
    }

    #[test]
    fn positioned_block_in_inline_preserves_native_unsupported_semantics() {
        let mut inline = element(1, None, "span", "auto");
        computed(&mut inline)[0] = property("display", "inline");
        let mut block = element(3, Some(1), "div", "40px");
        computed(&mut block).push(property("position", "relative"));
        let data = request(
            vec![
                inline,
                text(2, 1, &units("before")),
                block,
                text(4, 1, &units("after")),
            ],
            vec![1],
        );
        match measure_scene(&data) {
            Ok(result) => assert!(matches!(
                result.measurements[0].result,
                SceneResult::Unavailable { .. }
            )),
            Err(SceneError::Layout { message }) => assert!(message.contains("inline"), "{message}"),
            other => panic!("unexpected inline outcome: {other:?}"),
        }
    }

    #[test]
    fn no_requested_provider_values_need_no_fonts_or_layout() {
        let mut data = request(Vec::new(), Vec::new());
        data.fonts = SceneFonts::NativeHtmlSystem;
        let result = measure_scene(&data).unwrap();
        assert!(result.measurements.is_empty());
        assert_eq!(result.world, data.world);
    }
}
