//! Native source items retain unsnapped glyph positions but admit their aggregate
//! advance to the physical `LayoutUnit` grid before wrapping and placement. An item
//! is a run of one source at one bidi level (Chrome splits its items at level changes,
//! miaskiewicz/turbo-dom#238).
use super::{native_text_advance, NodeId, Seg, Word};

#[derive(Clone)]
pub(super) struct Cursor {
    source: Option<(NodeId, u8)>,
    origin: f64,
    advance: f64,
    units: f64,
}
impl Cursor {
    pub(super) fn new(scale: f32, origin: f32) -> Self {
        Self {
            source: None,
            origin: f64::from(origin),
            advance: 0.0,
            units: f64::from(scale) * 64.0,
        }
    }
    pub(super) fn position(&self) -> f32 {
        (self.origin + self.advance) as f32
    }
    pub(super) fn snapped(&self) -> f32 {
        (self.origin + (self.advance * self.units).ceil() / self.units) as f32
    }
    pub(super) fn begin(&mut self, source: (NodeId, u8)) {
        if self.source != Some(source) {
            self.barrier(0.0);
            self.source = Some(source);
        }
    }
    pub(super) fn text(&mut self, source: (NodeId, u8), advance: f64) {
        self.begin(source);
        self.advance += advance;
    }
    pub(super) fn barrier(&mut self, advance: f32) {
        self.origin += (self.advance * self.units).ceil() / self.units + f64::from(advance);
        self.advance = 0.0;
        self.source = None;
    }
    pub(super) fn word(&mut self, word: &Word) {
        if word.atom.is_some() {
            self.barrier(word.width);
            return;
        }
        for index in 0..=word.segs.len() {
            for (_, edge) in word.edges.iter().filter(|(at, _)| *at == index) {
                self.barrier(edge.advance);
            }
            if let Some(seg) = word.segs.get(index) {
                self.text((seg.node_id, seg.level), seg.native_width);
            }
        }
    }
    pub(super) fn space(&mut self, word: &Word) {
        if let Some((source, advance)) = word.native_space {
            self.text((source, word.space_level.unwrap_or(0)), advance);
        } else if word.space_after != 0.0 {
            self.barrier(word.space_after);
        }
    }
    pub(super) fn candidate_width(&self, word: &Word) -> f32 {
        let mut candidate = self.clone();
        candidate.word(word);
        candidate.snapped()
    }
}

fn prepare_segment(seg: &mut Seg, scale: f32) {
    if seg.upright {
        // Upright glyphs advance by the vertical advance, already `native_width`.
        return;
    }
    seg.native_width = native_text_advance::measure(
        &seg.face,
        (seg.font_size, scale),
        seg.letter_spacing,
        &seg.text,
        seg.level % 2 == 1,
        seg.original_units.as_deref(),
    );
}
fn prepare_word(word: &mut Word, scale: f32) {
    for seg in &mut word.segs {
        prepare_segment(seg, scale);
    }
    let source = word.spaces.first().or(word.advance_space.as_ref());
    if let Some(source) = source.filter(|_| word.space_after != 0.0) {
        word.native_space = Some((
            source.node_id,
            native_text_advance::measure(
                &source.face,
                (source.font_size, scale),
                source.letter_spacing,
                " ",
                false,
                None,
            ),
        ));
    }
}
pub(super) fn prepare(words: &mut [Word], scale: f32) {
    for word in words.iter_mut() {
        prepare_word(word, scale);
    }
    for index in 1..words.len() {
        let (head, tail) = words.split_at_mut(index);
        let word = &mut head[index - 1];
        let Some([before, middle, after]) = space_kerning(word, &tail[0], scale) else {
            continue;
        };
        if let Some(last) = word.segs.last_mut() {
            last.native_width += before;
            last.trailing_kern = before;
        }
        if let Some((_, advance)) = word.native_space.as_mut() {
            *advance += middle;
        }
        if let Some(first) = tail[0].segs.first_mut() {
            first.native_width += after;
            first.leading_kern = after;
        }
    }
}

// Chrome shapes a whole text item, so kerning pairs across a collapsible space adjust
// advances that separately shaped words lack. GPOS puts a pair's whole value on its
// first glyph: the `,`+space pair lands on the word's last glyph (a Range over the word
// includes it), the space+`W` pair on the space. A `kern`-table pair is split between its
// two glyphs (`FontFace::blink_advances`), so the space+`W` pair also moves the next
// word's first glyph. Applied only when the word's last segment, the space and the next
// word's first segment are one run (same source and font): a run boundary is a shaping
// boundary in Chrome too. Returns the `[last glyph, space, next first glyph]` deltas in
// px, from shaping `"{last} {first}"` against each character alone (each one glyph).
fn space_kerning_pair<'a>(word: &'a Word, next: &'a Word) -> Option<(&'a Seg, char, char)> {
    let space = word.spaces.first().or(word.advance_space.as_ref())?;
    word.native_space?;
    let (last, first) = (word.segs.last()?, next.segs.first()?);
    if !same_run(last, space) || !same_run(space, first) {
        return None;
    }
    let before = edge_char(last, true)?;
    let after = edge_char(first, false)?;
    Some((space, before, after))
}

fn space_kerning(word: &Word, next: &Word, scale: f32) -> Option<[f64; 3]> {
    let (space, before, after) = space_kerning_pair(word, next)?;
    let advances = |text: &str| {
        let glyphs = space.face.shape(text);
        native_text_advance::advances(&space.face, (space.font_size, scale), &glyphs, 0.0)
    };
    let joined = advances(&format!("{before} {after}"));
    let alone = [before, ' ', after].map(|c| advances(&c.to_string()));
    if joined.len() != 3 || alone.iter().any(|a| a.len() != 1) {
        return None;
    }
    Some([0, 1, 2].map(|i| joined[i] - alone[i][0]))
}

fn same_run(a: &Seg, b: &Seg) -> bool {
    a.node_id == b.node_id
        && a.font_size == b.font_size
        && a.face.data().as_ptr() == b.face.data().as_ptr()
        && a.face.index() == b.face.index()
}

/// The segment's last (`end`) or first character as it is shaped: from its original
/// UTF-16 units when it has them (an unpaired surrogate there yields `None`).
fn edge_char(seg: &Seg, end: bool) -> Option<char> {
    match &seg.original_units {
        Some(units) => {
            let mut chars = char::decode_utf16(units.iter().copied());
            let edge = if end { chars.last() } else { chars.next() };
            edge?.ok()
        }
        None if end => seg.text.chars().next_back(),
        None => seg.text.chars().next(),
    }
}
pub(super) fn line_width(words: &[Word], scale: f32) -> f32 {
    let mut cursor = Cursor::new(scale, 0.0);
    for (index, word) in words.iter().enumerate() {
        cursor.word(word);
        if index + 1 < words.len() {
            cursor.space(word);
        }
    }
    cursor.snapped()
}

#[cfg(test)]
mod tests {
    use super::super::{atom_word, break_word, place_line, InlineAtom, VAlign};
    use crate::text::Align;

    #[test]
    fn numeric_spacing_without_source_metadata_preserves_the_next_atomic_placement() {
        // A legacy item can carry a numeric gap without a text-source record.
        // Retain that compatibility advance without manufacturing glyph owners.
        let mut gap = break_word();
        gap.break_after = false;
        gap.space_after = 3.25;
        let next = InlineAtom {
            id: 17,
            width: 10.0,
            height: 10.0,
            valign: VAlign::Baseline,
            font_size: 16.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
            placeholder: false,
            baseline: None,
        };
        let placed = place_line(
            vec![gap, atom_word(next, 0.0)],
            100.0,
            Align::Left,
            24.0,
            &[],
            (None, false),
            (true, true, super::super::NativeScale::both(Some(2.0))),
        );
        assert_eq!(placed.line.atoms.len(), 1);
        assert_eq!(placed.line.atoms[0].id, 17);
        assert_eq!(placed.line.atoms[0].x, 3.25);
        assert_eq!(placed.line.width, 13.25);
        assert_eq!(placed.line.runs.len(), 0);
        assert_eq!(placed.clusters.len(), 0);
    }
}
