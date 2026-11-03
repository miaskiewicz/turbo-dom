//! UAX #9 bidi for inline layout (Chrome 156, miaskiewicz/turbo-dom#238).
//!
//! A paragraph's text, with each inline box's `unicode-bidi` / `direction` as the
//! explicit formatting characters Chrome inserts for it (an atom as U+FFFC, a forced
//! break as a paragraph separator), resolves to embedding levels once per paragraph
//! ([`paragraph_levels`]); the shaping segments split where the level changes, so each
//! is shaped in one direction. Lines break in logical order; each line is then
//! reordered (rule L2) by [`reorder_line`],
//! which moves the placed glyph runs and atoms to their visual positions and puts each
//! inline box's borders, padding and margins on its visual sides: its physical left
//! decoration before its visually first fragment and its right one after its last. A
//! box splits into one fragment per visually contiguous run of its content, and (an
//! embedding / isolate / override box) where its content's level changes, as Chrome's.

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;

use unicode_bidi::{bidi_class, BidiClass, BidiInfo, Level};

use super::{InlineEdge, NodeId, PlacedAtom, PlacedInlineEdge, Piece, SourceRun};

/// The `unicode-bidi` of an inline box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnicodeBidi {
    #[default]
    Normal,
    Embed,
    Isolate,
    BidiOverride,
    IsolateOverride,
    Plaintext,
}

impl UnicodeBidi {
    /// The computed value of a `unicode-bidi` declaration (`normal` when absent or
    /// unknown).
    #[must_use]
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(v) if v.eq_ignore_ascii_case("embed") => Self::Embed,
            Some(v) if v.eq_ignore_ascii_case("isolate") => Self::Isolate,
            Some(v) if v.eq_ignore_ascii_case("bidi-override") => Self::BidiOverride,
            Some(v) if v.eq_ignore_ascii_case("isolate-override") => Self::IsolateOverride,
            Some(v) if v.eq_ignore_ascii_case("plaintext") => Self::Plaintext,
            _ => Self::Normal,
        }
    }

    /// The formatting characters opening and closing a box's content.
    pub(crate) fn controls(self, rtl: bool) -> (&'static str, &'static str) {
        match (self, rtl) {
            (Self::Normal, _) => ("", ""),
            (Self::Embed, false) => ("\u{202a}", "\u{202c}"),
            (Self::Embed, true) => ("\u{202b}", "\u{202c}"),
            (Self::Isolate, false) => ("\u{2066}", "\u{2069}"),
            (Self::Isolate, true) => ("\u{2067}", "\u{2069}"),
            // Chrome resolves isolate-override content one level in, as an override.
            (Self::BidiOverride | Self::IsolateOverride, false) => ("\u{202d}", "\u{202c}"),
            (Self::BidiOverride | Self::IsolateOverride, true) => ("\u{202e}", "\u{202c}"),
            (Self::Plaintext, _) => ("\u{2068}", "\u{2069}"),
        }
    }
}

thread_local! {
    /// The base direction of the paragraph being laid out: `Some(rtl)`, or `None` for
    /// `unicode-bidi: plaintext` (each paragraph's first strong character, P2/P3).
    static BASE: Cell<Option<bool>> = const { Cell::new(Some(false)) };
}

/// Restores the previous paragraph base direction when dropped.
pub(crate) struct BaseScope(Option<bool>);

impl Drop for BaseScope {
    fn drop(&mut self) {
        BASE.with(|base| base.set(self.0));
    }
}

/// Lay paragraphs out with base direction `base` until the guard drops.
pub(crate) fn base_scope(base: Option<bool>) -> BaseScope {
    BaseScope(BASE.with(|cell| cell.replace(base)))
}

/// An explicit bidi formatting character (embedding, override, isolate or their
/// terminators): it has no glyph and no advance.
#[must_use]
pub fn is_formatting(c: char) -> bool {
    matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Whether the first strong character of `text` (P2, skipping nothing) is
/// right-to-left; `None` when it has none.
#[must_use]
pub fn first_strong_rtl(text: &str) -> Option<bool> {
    text.chars().find_map(|c| match bidi_class(c) {
        BidiClass::L => Some(false),
        BidiClass::R | BidiClass::AL => Some(true),
        _ => None,
    })
}

/// The resolved levels of one paragraph: per text run (in piece order) one level per
/// byte of its text and its paragraph's level, and per atom (in piece order) its
/// level.
#[derive(Debug, Default)]
pub(crate) struct ParagraphLevels {
    pub runs: Vec<Vec<u8>>,
    pub paragraph: Vec<u8>,
    pub atoms: Vec<u8>,
    /// The paragraph needs bidi processing: it holds right-to-left content or an RTL
    /// base / inline box. Otherwise every level is 0 and lines keep logical order.
    pub active: bool,
}

fn needs_bidi(c: char) -> bool {
    matches!(
        bidi_class(c),
        BidiClass::R
            | BidiClass::AL
            | BidiClass::AN
            | BidiClass::RLE
            | BidiClass::RLO
            | BidiClass::RLI
    )
}

/// Resolve the embedding levels of a paragraph of `pieces` under the current base
/// direction ([`base_scope`]).
pub(crate) fn paragraph_levels(pieces: &[Piece]) -> ParagraphLevels {
    let base = BASE.with(Cell::get);
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut atoms = Vec::new();
    let mut active = base != Some(false);
    for piece in pieces {
        match piece {
            Piece::Run(run) => {
                active |= run.text.chars().any(needs_bidi);
                runs.push(text.len()..text.len() + run.text.len());
                text.push_str(&run.text);
            }
            Piece::Edge(edge) => {
                active |= edge.rtl;
                let (open, close) = edge.bidi.controls(edge.rtl);
                text.push_str(if edge.start { open } else { close });
            }
            Piece::Atom(_) | Piece::NativeAtom(..) => {
                atoms.push(text.len());
                text.push('\u{fffc}');
            }
            Piece::SourceBreak(_) | Piece::Break => text.push('\n'),
        }
    }
    if !active {
        return ParagraphLevels {
            runs: runs.iter().map(|range| vec![0; range.len()]).collect(),
            paragraph: vec![0; runs.len()],
            atoms: vec![0; atoms.len()],
            active,
        };
    }
    let info = BidiInfo::new(
        &text,
        base.map(|rtl| if rtl { Level::rtl() } else { Level::ltr() }),
    );
    let level = |byte: usize| info.levels.get(byte).map_or(0, Level::number);
    let paragraph_of = |byte: usize| {
        info.paragraphs
            .iter()
            .find(|p| p.range.contains(&byte))
            .map_or(0, |p| p.level.number())
    };
    ParagraphLevels {
        paragraph: runs.iter().map(|range| paragraph_of(range.start)).collect(),
        runs: runs
            .iter()
            .map(|range| range.clone().map(level).collect())
            .collect(),
        atoms: atoms.into_iter().map(level).collect(),
        active,
    }
}

/// One placed item of a line in logical order: a shaping segment, an atom, the
/// white space after a word, or an empty inline box's place.
#[derive(Debug, Clone)]
pub(super) struct LineItem {
    pub x0: f32,
    pub x1: f32,
    pub level: u8,
    /// The inline boxes containing it, outermost first.
    pub boxes: Vec<NodeId>,
    pub runs: Range<usize>,
    pub atom: Option<usize>,
    /// The white space after a word (a line ending in it hangs it).
    pub space: bool,
}

/// The visual order of `levels` (rule L2): from the highest level down to the lowest
/// odd one, every maximal run at that level or above reverses.
fn visual_order(levels: &[u8]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..levels.len()).collect();
    let Some(&max) = levels.iter().max() else {
        return order;
    };
    let min = levels.iter().copied().min().unwrap_or(0);
    let lowest_odd = if min % 2 == 1 { min } else { min + 1 };
    let mut level = max;
    while level >= lowest_odd {
        let mut start = 0;
        while start < order.len() {
            if levels[order[start]] < level {
                start += 1;
                continue;
            }
            let end = (start..order.len())
                .find(|&i| levels[order[i]] < level)
                .unwrap_or(order.len());
            order[start..end].reverse();
            start = end;
        }
        level -= 1;
    }
    order
}

/// One inline box on a line: its direction, `unicode-bidi`, edges and a
/// representative edge (for its font metrics).
struct LineBox {
    rtl: bool,
    bidi: UnicodeBidi,
    start: Option<f32>,
    end: Option<f32>,
    edge: InlineEdge,
    depth: usize,
}

impl LineBox {
    /// The decoration on the box's physical left side, when it is on this line.
    fn left(&self) -> Option<f32> {
        if self.rtl {
            self.end
        } else {
            self.start
        }
    }
    fn right(&self) -> Option<f32> {
        if self.rtl {
            self.start
        } else {
            self.end
        }
    }
}

/// A visually contiguous fragment of one box: visual positions `first..=last`.
struct BoxPiece {
    node: NodeId,
    first: usize,
    last: usize,
    left: bool,
    right: bool,
}

fn line_boxes(
    items: &[LineItem],
    open: &[InlineEdge],
    edges: &[PlacedInlineEdge],
) -> HashMap<NodeId, LineBox> {
    let mut boxes: HashMap<NodeId, LineBox> = HashMap::new();
    let mut add = |edge: &InlineEdge, depth: usize| {
        boxes.entry(edge.node_id).or_insert_with(|| LineBox {
            rtl: edge.rtl,
            bidi: edge.bidi,
            start: None,
            end: None,
            edge: edge.clone(),
            depth,
        });
    };
    for (depth, edge) in open.iter().enumerate() {
        add(edge, depth);
    }
    for item in items {
        for (depth, node) in item.boxes.iter().enumerate() {
            if let Some(edge) = edges.iter().find(|e| e.edge.node_id == *node) {
                add(&edge.edge, depth);
            }
        }
    }
    for placed in edges {
        if let Some(line_box) = boxes.get_mut(&placed.edge.node_id) {
            if placed.edge.start {
                line_box.start = Some(placed.edge.advance);
            } else {
                line_box.end = Some(placed.edge.advance);
            }
        }
    }
    boxes
}

/// Split each box's members (visual positions) into its fragments.
fn box_pieces(
    order: &[usize],
    (items, levels): (&[LineItem], &[u8]),
    boxes: &HashMap<NodeId, LineBox>,
) -> Vec<BoxPiece> {
    let mut pieces: Vec<BoxPiece> = Vec::new();
    let mut nodes: Vec<&NodeId> = boxes.keys().collect();
    nodes.sort_by_key(|node| (boxes[node].depth, node.0));
    for node in nodes {
        let line_box = &boxes[node];
        let splits = line_box.bidi != UnicodeBidi::Normal;
        let mut own: Vec<BoxPiece> = Vec::new();
        for (position, &index) in order.iter().enumerate() {
            if !items[index].boxes.contains(node) {
                continue;
            }
            let joins = own.last().is_some_and(|piece| {
                piece.last + 1 == position
                    && !(splits && levels[order[piece.last]] != levels[index])
            });
            match own.last_mut() {
                Some(piece) if joins => piece.last = position,
                _ => own.push(BoxPiece {
                    node: *node,
                    first: position,
                    last: position,
                    left: false,
                    right: false,
                }),
            }
        }
        if let Some(first) = own.first_mut() {
            first.left = line_box.left().is_some();
        }
        if let Some(last) = own.last_mut() {
            last.right = line_box.right().is_some();
        }
        pieces.extend(own);
    }
    pieces
}

/// The placed line `runs` / `atoms` / `edges` (logical order, starting at `pen`)
/// reordered visually: items move, and the inline edges are replaced by each box
/// fragment's start (its left side) and end (its right side) edges.
pub(super) fn reorder_line(
    mut items: Vec<LineItem>,
    pen: f32,
    open: &[InlineEdge],
    placed: (&mut [SourceRun], &mut [PlacedAtom], &mut Vec<PlacedInlineEdge>),
) {
    let (runs, atoms, edges) = placed;
    // Not L1: Chrome hangs a line's trailing collapsible space next to the character
    // before it, at that character's level and in its boxes.
    if let Some(last) = items.iter().rposition(|item| !item.space) {
        let (level, boxes) = (items[last].level, items[last].boxes.clone());
        for item in &mut items[last + 1..] {
            item.level = level;
            item.boxes.clone_from(&boxes);
        }
    }
    let levels: Vec<u8> = items.iter().map(|item| item.level).collect();
    let items = &items[..];
    let order = visual_order(&levels);
    let boxes = line_boxes(items, open, edges);
    let pieces = box_pieces(&order, (items, &levels), &boxes);
    let mut visual = Vec::new();
    let mut x = pen;
    for (position, &index) in order.iter().enumerate() {
        let mut starting: Vec<&BoxPiece> = pieces.iter().filter(|p| p.first == position).collect();
        starting.sort_by_key(|p| boxes[&p.node].depth);
        for piece in starting {
            let line_box = &boxes[&piece.node];
            let advance = if piece.left {
                line_box.left().unwrap_or(0.0)
            } else {
                0.0
            };
            visual.push(visual_edge(line_box, true, x, advance, piece.left));
            x += advance;
        }
        let item = &items[index];
        let dx = x - item.x0;
        for run in &mut runs[item.runs.clone()] {
            shift_run(run, dx);
        }
        if let Some(atom) = item.atom {
            atoms[atom].x += dx;
        }
        x += item.x1 - item.x0;
        let mut ending: Vec<&BoxPiece> = pieces.iter().filter(|p| p.last == position).collect();
        ending.sort_by_key(|p| std::cmp::Reverse(boxes[&p.node].depth));
        for piece in ending {
            let line_box = &boxes[&piece.node];
            let advance = if piece.right {
                line_box.right().unwrap_or(0.0)
            } else {
                0.0
            };
            visual.push(visual_edge(line_box, false, x, advance, piece.right));
            x += advance;
        }
    }
    *edges = visual;
}

fn visual_edge(line_box: &LineBox, start: bool, x: f32, advance: f32, decorated: bool) -> PlacedInlineEdge {
    let mut edge = line_box.edge.clone();
    edge.start = start;
    edge.advance = advance;
    PlacedInlineEdge { edge, x, decorated }
}

fn shift_run(run: &mut SourceRun, dx: f32) {
    if dx == 0.0 {
        return;
    }
    for glyph in &mut run.run.glyphs {
        glyph.x += dx;
    }
    for cluster in &mut run.clusters {
        cluster.pen_x += dx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_reverses_from_the_highest_level_to_the_lowest_odd_one() {
        assert_eq!(visual_order(&[0, 0, 1, 1, 0]), [0, 1, 3, 2, 4]);
        assert_eq!(visual_order(&[1, 2, 2, 1]), [3, 1, 2, 0]);
        assert_eq!(visual_order(&[0, 1, 2, 1, 0]), [0, 3, 2, 1, 4]);
        assert_eq!(visual_order(&[2, 2]), [0, 1]);
        assert_eq!(visual_order(&[]), Vec::<usize>::new());
    }

    #[test]
    fn unicode_bidi_parses_its_keywords_and_opens_its_controls() {
        for (value, parsed) in [
            ("embed", UnicodeBidi::Embed),
            (" Isolate ", UnicodeBidi::Isolate),
            ("bidi-override", UnicodeBidi::BidiOverride),
            ("isolate-override", UnicodeBidi::IsolateOverride),
            ("plaintext", UnicodeBidi::Plaintext),
            ("normal", UnicodeBidi::Normal),
            ("bogus", UnicodeBidi::Normal),
        ] {
            assert_eq!(UnicodeBidi::parse(Some(value)), parsed, "{value}");
        }
        assert_eq!(UnicodeBidi::parse(None), UnicodeBidi::Normal);
        assert_eq!(UnicodeBidi::Embed.controls(false), ("\u{202a}", "\u{202c}"));
        assert_eq!(
            UnicodeBidi::IsolateOverride.controls(true),
            ("\u{202e}", "\u{202c}")
        );
        assert_eq!(UnicodeBidi::Plaintext.controls(true), ("\u{2068}", "\u{2069}"));
    }

    #[test]
    fn first_strong_direction() {
        assert_eq!(first_strong_rtl("12 \u{5d0}b"), Some(true));
        assert_eq!(first_strong_rtl("(a\u{5d0}"), Some(false));
        assert_eq!(first_strong_rtl("123 !"), None);
    }
}
