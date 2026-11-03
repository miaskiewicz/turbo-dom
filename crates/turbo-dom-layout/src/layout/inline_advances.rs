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

fn prepare_segment(seg: &mut Seg) {
    seg.native_width = native_text_advance::measure(
        &seg.face,
        (seg.font_size, seg.letter_spacing),
        &seg.text,
        seg.level % 2 == 1,
        seg.original_units.as_deref(),
    );
}
fn prepare_word(word: &mut Word) {
    word.segs.iter_mut().for_each(prepare_segment);
    let source = word.spaces.first().or(word.advance_space.as_ref());
    if let Some(source) = source.filter(|_| word.space_after != 0.0) {
        word.native_space = Some((
            source.node_id,
            native_text_advance::measure(
                &source.face,
                (source.font_size, source.letter_spacing),
                " ",
                false,
                None,
            ),
        ));
    }
}
pub(super) fn prepare(words: &mut [Word]) {
    words.iter_mut().for_each(prepare_word);
    for index in 1..words.len() {
        let (head, tail) = words.split_at_mut(index);
        let word = &mut head[index - 1];
        let Some([before, after]) = space_kerning(word, &tail[0]) else {
            continue;
        };
        if let Some(last) = word.segs.last_mut() {
            last.native_width += before;
            last.trailing_kern = before;
        }
        if let Some((_, advance)) = word.native_space.as_mut() {
            *advance += after;
        }
    }
}

// Chrome shapes a whole text item, so GPOS/`kern` pairs across a collapsible space
// adjust advances that separately shaped words lack: the `,`+space pair lands on the
// word's last glyph (a Range over the word includes it), the space+`W` pair on the
// space. Applied only when the word's last segment, the space and the next word's
// first segment are one run (same source and font): a run boundary is a shaping
// boundary in Chrome too. Returns `[before, after]` in px.
fn space_kerning(word: &Word, next: &Word) -> Option<[f64; 2]> {
    let space = word.spaces.first().or(word.advance_space.as_ref())?;
    word.native_space?;
    let (last, first) = (word.segs.last()?, next.segs.first()?);
    if !same_run(last, space) || !same_run(space, first) {
        return None;
    }
    let (before, after) = (edge_char(last, true)?, edge_char(first, false)?);
    let measure =
        |text: &str| {
            native_text_advance::measure(&space.face, (space.font_size, 0.0), text, false, None)
        };
    let gap = measure(" ");
    Some([
        measure(&format!("{before} ")) - measure(&before.to_string()) - gap,
        measure(&format!(" {after}")) - measure(&after.to_string()) - gap,
    ])
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
            (true, true, Some(2.0)),
        );
        assert_eq!(placed.line.atoms.len(), 1);
        assert_eq!(placed.line.atoms[0].id, 17);
        assert_eq!(placed.line.atoms[0].x, 3.25);
        assert_eq!(placed.line.width, 13.25);
        assert_eq!(placed.line.runs.len(), 0);
        assert_eq!(placed.clusters.len(), 0);
    }
}
