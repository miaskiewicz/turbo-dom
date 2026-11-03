//! Form-state pseudo-classes for the selector APIs (querySelector[All], matches, closest).
//!
//! rtdom's matcher only knows attribute-level approximations of these states (and lacks
//! `:indeterminate`, `:default`, `:placeholder-shown`, the range and validity classes). Their
//! truth lives in the binding: IDL-only indeterminate flags, checkedness and selectedness,
//! constraint validation and the user-validity flag a submission attempt sets. As
//! `:popover-open` and `:modal` do, each occurrence is replaced by structural selectors for the
//! elements of the queried root that match, classified by the private helper captured from
//! form_pseudo.js. `:read-only` is the complement of `:read-write`. The same transform answers
//! `:scope`, which rtdom does not know: the queried element itself, the document element for a
//! document, and nothing for a fragment.
use super::*;

struct Classifier(v8::Weak<v8::Function>);

const NAMES: [&str; 16] = [
    "checked",
    "indeterminate",
    "default",
    "placeholder-shown",
    "read-only",
    "read-write",
    "in-range",
    "out-of-range",
    "valid",
    "invalid",
    "user-valid",
    "user-invalid",
    "enabled",
    "disabled",
    "required",
    "optional",
];

fn pseudo_name(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("scope") {
        return Some("scope");
    }
    NAMES
        .iter()
        .copied()
        .find(|known| known.eq_ignore_ascii_case(name))
}

fn scope_selector(tree: &Tree, anchor: Handle) -> String {
    match tree.node_type_id(anchor) {
        1 => format!(":is({})", html_popover::selector_path(tree, anchor)),
        9 => ":root".to_owned(),
        _ => ":not(*)".to_owned(),
    }
}

/// Byte ranges of `:name` occurrences, inside functional arguments too.
fn pseudo_ranges<'i>(
    input: &mut cssparser::Parser<'i, '_>,
    ranges: &mut Vec<(usize, usize, &'static str)>,
) -> Result<(), cssparser::ParseError<'i, ()>> {
    use cssparser::Token;
    let mut colon = None;
    let mut double_colon = false;
    while !input.is_exhausted() {
        let position = input.position().byte_index();
        let token = input.next_including_whitespace_and_comments()?.clone();
        match token {
            Token::Colon => {
                double_colon = colon.is_some();
                colon = if double_colon { None } else { Some(position) };
            }
            Token::Ident(ref name) => {
                if let (false, Some(start), Some(pseudo)) = (double_colon, colon, pseudo_name(name))
                {
                    ranges.push((start, input.position().byte_index(), pseudo));
                }
                colon = None;
                double_colon = false;
            }
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                colon = None;
                double_colon = false;
                input.parse_nested_block(|nested| pseudo_ranges(nested, ranges))?;
            }
            Token::Comment(_) => (),
            _ => {
                colon = None;
                double_colon = false;
            }
        }
    }
    Ok(())
}

/// Every element of `root`'s tree (the root itself when it is one), in tree order.
fn tree_elements(tree: &Tree, root: Handle) -> Vec<Handle> {
    let mut elements = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if tree.node_type_id(node) == 1 {
            elements.push(node);
        }
        stack.extend(tree.children(node).into_iter().rev());
    }
    elements
}

/// Pseudo-classes whose state is the element's own: classifying an element needs no other
/// element of the tree. (`:indeterminate` reads the radio group, `:valid`/`:invalid` and the
/// user-validity classes a form's or fieldset's controls.)
const ELEMENT_LOCAL: [&str; 12] = [
    "checked",
    "default",
    "placeholder-shown",
    "read-only",
    "read-write",
    "in-range",
    "out-of-range",
    "enabled",
    "disabled",
    "required",
    "optional",
    "scope",
];

/// Pseudo-classes whose state is a form's or fieldset's controls' (`:valid`, `:invalid`) or
/// the element's own (the user-validity classes match controls only).
const VALIDITY: [&str; 4] = ["valid", "invalid", "user-valid", "user-invalid"];

/// The elements a `matches()`/`closest()` answer for `anchor` can read the pseudo-classes of:
/// its inclusive ancestors (every compound of a selector without `:has()` matches one of them
/// or one of their preceding siblings, the latter only through `+`/`~`) and, for the validity
/// classes, the controls that make one of them a valid or invalid form or fieldset. `None` when
/// the selector or a class needs the whole tree.
fn subject_elements(
    tree: &Tree,
    anchor: Handle,
    source: &str,
    names: &[&str],
) -> Option<Vec<Handle>> {
    if source.to_ascii_lowercase().contains(":has(")
        || !names
            .iter()
            .all(|name| ELEMENT_LOCAL.contains(name) || VALIDITY.contains(name))
    {
        return None;
    }
    let siblings = source.contains('+') || source.contains('~');
    let mut elements = Vec::new();
    let mut node = Some(anchor);
    while let Some(current) = node {
        if tree.node_type_id(current) == 1 {
            elements.push(current);
        }
        if siblings {
            let mut sibling = tree.previous_sibling(current);
            while let Some(previous) = sibling {
                if tree.node_type_id(previous) == 1 {
                    elements.push(previous);
                }
                sibling = tree.previous_sibling(previous);
            }
        }
        node = tree.parent(current);
    }
    if names.iter().any(|name| VALIDITY.contains(name)) {
        add_validity_controls(tree, anchor, &mut elements);
    }
    Some(elements)
}

/// A listed element, or one that may be a form-associated custom element.
fn maybe_control(tree: &Tree, node: Handle) -> bool {
    tree.namespace_id(node) == 0
        && tree.local_name(node).is_some_and(|name| {
            matches!(
                name,
                "input" | "button" | "select" | "textarea" | "object" | "output"
            ) || name.contains('-')
        })
}

/// The controls whose validity a form or fieldset among `elements` reads: a fieldset's
/// descendant controls, and a form's owned ones (its descendants, and anywhere in the tree the
/// controls naming a form with their `form` attribute).
fn add_validity_controls(tree: &Tree, anchor: Handle, elements: &mut Vec<Handle>) {
    let containers: Vec<(Handle, bool)> = elements
        .iter()
        .filter(|&&node| tree.namespace_id(node) == 0)
        .filter_map(|&node| match tree.local_name(node) {
            Some("form") => Some((node, true)),
            Some("fieldset") => Some((node, false)),
            _ => None,
        })
        .collect();
    let mut seen: std::collections::HashSet<Handle> = elements.iter().copied().collect();
    let mut add = |node: Handle, elements: &mut Vec<Handle>| {
        if maybe_control(tree, node) && seen.insert(node) {
            elements.push(node);
        }
    };
    for &(container, _) in &containers {
        for node in tree.descendants(container) {
            add(node, elements);
        }
    }
    if containers.iter().any(|&(_, form)| form) {
        for node in tree.descendants(html_popover::selector_root(tree, anchor)) {
            if tree.has_attribute(node, "form") {
                add(node, elements);
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Test hook: how many elements the classifier was handed.
    pub(crate) static CLASSIFIED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The matching elements of `root` (of `candidates` when given) for each requested
/// pseudo-class, or `None` when this realm has no classifier (the selector then keeps rtdom's
/// own matching).
fn classify(
    scope: &mut v8::PinScope,
    root: Handle,
    candidates: Option<Vec<Handle>>,
    names: &[&'static str],
) -> Option<Vec<Vec<Handle>>> {
    let classifier = scope
        .get_current_context()
        .get_slot::<Classifier>()
        .and_then(|slot| slot.0.to_local(scope))?;
    let handles = match candidates {
        Some(handles) => handles,
        None => with_tree(|tree| tree_elements(tree, root))?,
    };
    #[cfg(test)]
    CLASSIFIED.with(|count| count.set(count.get() + handles.len()));
    let elements: Vec<v8::Local<v8::Value>> = handles
        .iter()
        .map(|&handle| wrap(scope, handle).into())
        .collect();
    let elements = v8::Array::new_with_elements(scope, &elements);
    let requested: Vec<v8::Local<v8::Value>> = names
        .iter()
        // invariant: engine-built name, far below V8's string length limit
        .map(|name| v8::String::new(scope, name).unwrap().into())
        .collect();
    let requested = v8::Array::new_with_elements(scope, &requested);
    let receiver = v8::undefined(scope).into();
    let result = classifier.call(scope, receiver, &[elements.into(), requested.into()])?;
    let result = v8::Local::<v8::Array>::try_from(result).ok()?;
    let mut sets = Vec::with_capacity(names.len());
    for index in 0..names.len() as u32 {
        let matched = result
            .get_index(scope, index)
            .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())?;
        let mut set = Vec::with_capacity(matched.length() as usize);
        for at in 0..matched.length() {
            let element = matched
                .get_index(scope, at)
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())?;
            set.push(handle_of(scope, element)?);
        }
        sets.push(set);
    }
    Some(sets)
}

/// `subject_local`: the query is `matches()`/`closest()` on `anchor`, whose answer depends only
/// on `anchor`'s ancestors (and their siblings), not on the rest of the tree.
pub(super) fn selector(
    scope: &mut v8::PinScope,
    anchor: Handle,
    source: &str,
    subject_local: bool,
) -> Option<String> {
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    if pseudo_ranges(&mut parser, &mut ranges).is_err() || ranges.is_empty() {
        return Some(source.to_owned());
    }
    let mut names: Vec<&'static str> = Vec::new();
    for &(_, _, name) in ranges.iter().filter(|range| range.2 != "scope") {
        // `:read-only` is answered by its complement.
        let name = if name == "read-only" {
            "read-write"
        } else {
            name
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let (root, scoped) = with_tree(|tree| {
        (
            html_popover::selector_root(tree, anchor),
            scope_selector(tree, anchor),
        )
    })?;
    let sets = if names.is_empty() {
        Vec::new()
    } else {
        let candidates = subject_local
            .then(|| with_tree(|tree| subject_elements(tree, anchor, source, &names)))
            .flatten()
            .flatten();
        match classify(scope, root, candidates, &names) {
            Some(sets) => sets,
            None => return Some(source.to_owned()),
        }
    };
    let replacements: Vec<(&'static str, String)> = with_tree(|tree| {
        names
            .iter()
            .zip(&sets)
            .map(|(&name, set)| {
                let paths: Vec<String> = set
                    .iter()
                    .map(|&node| html_popover::selector_path(tree, node))
                    .collect();
                (name, paths.join(","))
            })
            .collect()
    })?;
    let replacement = |name: &str| -> String {
        if name == "scope" {
            return scoped.clone();
        }
        let lookup = if name == "read-only" {
            "read-write"
        } else {
            name
        };
        let paths = &replacements
            .iter()
            .find(|(known, _)| *known == lookup)
            // invariant: value produced by engine code, never page input
            .unwrap()
            .1;
        match (name == "read-only", paths.is_empty()) {
            (false, true) => ":not(*)".to_owned(),
            (false, false) => format!(":is({paths})"),
            (true, true) => ":is(*)".to_owned(),
            (true, false) => format!(":not(:is({paths}))"),
        }
    };
    let mut result = String::new();
    let mut start = 0;
    for (from, to, name) in ranges {
        result.push_str(&source[start..from]);
        result.push_str(&replacement(name));
        start = to;
    }
    result.push_str(&source[start..]);
    Some(result)
}

pub(super) fn install(scope: &mut v8::PinScope) {
    run_js(scope, include_str!("form_pseudo.js"));
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__captureFormPseudo").unwrap();
    if let Some(callback) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(callback) = dom_ownership::helper_function(scope, callback) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(Classifier(callback)));
        }
    }
    global.delete(scope, key.into());
}
