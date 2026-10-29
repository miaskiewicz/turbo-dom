//! CSS `text-transform` (CSS Text 3 §2.1) as Chrome 156 applies it.
//!
//! `uppercase` / `lowercase` use full Unicode case mapping (SpecialCasing:
//! `ß` → `SS`, `ﬁ` → `FI`, final sigma) with the language-sensitive tailorings
//! Chrome applies from the element's `lang` (`-webkit-locale`): Turkish and
//! Azeri dotted/dotless i, Lithuanian dot above, and Greek accent removal in
//! uppercase. `capitalize` titlecases the first letter of each word with a
//! single-character mapping and ignores the language. Chrome 156 does not
//! support `full-width` (it has no effect), so neither do we.

/// Which case mapping a run's `text-transform` selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseTransform {
    #[default]
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

/// The language tailoring of a case mapping, from the content language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseLocale {
    #[default]
    Default,
    /// `tr` / `az`: `i` ↔ `İ`, `I` ↔ `ı`.
    Turkic,
    /// `lt`: the dot above of `i`/`j`/`į` is explicit.
    Lithuanian,
    /// `el`: uppercase drops accents and breathings.
    Greek,
}

/// A run's resolved `text-transform` and the content language it maps with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextTransform {
    pub case: CaseTransform,
    pub locale: CaseLocale,
}
