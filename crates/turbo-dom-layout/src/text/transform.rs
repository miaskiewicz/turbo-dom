//! CSS `text-transform` (CSS Text 3 §2.1) as Chrome 156 applies it.
//!
//! `uppercase` / `lowercase` use full Unicode case mapping (`SpecialCasing`:
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

impl TextTransform {
    /// The transform a computed `text-transform` / `-webkit-locale` pair selects.
    /// Only the single keywords Chrome 156 applies are recognised; anything else
    /// (`full-width`, a keyword list) leaves the text as authored.
    #[must_use]
    pub fn from_computed(text_transform: Option<&str>, locale: Option<&str>) -> Self {
        let case = match text_transform.map(str::trim) {
            Some("uppercase") => CaseTransform::Uppercase,
            Some("lowercase") => CaseTransform::Lowercase,
            Some("capitalize") => CaseTransform::Capitalize,
            _ => CaseTransform::None,
        };
        Self {
            case,
            locale: locale.map_or(CaseLocale::Default, CaseLocale::from_lang),
        }
    }
}

impl CaseLocale {
    /// The tailoring of a BCP 47 language tag (its primary subtag, any case).
    #[must_use]
    pub fn from_lang(tag: &str) -> Self {
        let tag = tag.trim().trim_matches(|c| c == '"' || c == '\'');
        let primary = tag.split(['-', '_']).next().unwrap_or("");
        match primary.to_ascii_lowercase().as_str() {
            "tr" | "az" => Self::Turkic,
            "lt" => Self::Lithuanian,
            "el" => Self::Greek,
            _ => Self::Default,
        }
    }
}

/// One output character of a transformed run and the source byte range it came
/// from. Every character of a multi-character mapping (`ß` → `SS`) carries the
/// whole source character; a source character a tailoring drops (the Turkish
/// `I` + U+0307 dot) widens the range of the output character before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mapped {
    pub ch: char,
    pub start: usize,
    pub end: usize,
}

/// Word context carried across the runs of one inline formatting context for
/// `capitalize` (Chrome 156: the preceding text of the line, not only the run).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WordContext {
    in_word: bool,
    after_alnum: bool,
}

impl WordContext {
    /// Advance past one source character.
    pub(crate) fn push(&mut self, ch: char) {
        if is_mark(ch) {
            return;
        }
        let word = is_word_char(ch);
        self.in_word = word || (self.after_alnum && is_mid_word(ch));
        self.after_alnum = word;
    }
}

/// Apply `transform` to `text`, given the word context before it (updated to
/// the end of `text`). `None` when the transform is `none`.
pub(crate) fn transform_run(
    text: &str,
    transform: TextTransform,
    context: &mut WordContext,
) -> Option<Vec<Mapped>> {
    if transform.case == CaseTransform::None {
        text.chars().for_each(|ch| context.push(ch));
        return None;
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out: Vec<Mapped> = Vec::with_capacity(text.len());
    for (index, &(start, ch)) in chars.iter().enumerate() {
        let end = start + ch.len_utf8();
        let prev = index.checked_sub(1).map(|i| chars[i].1);
        let mapped = match transform.case {
            CaseTransform::Capitalize => capitalize(ch, context.in_word),
            CaseTransform::Uppercase => Some(upper(ch, prev, transform.locale)),
            CaseTransform::Lowercase => Some(lower(&chars, index, transform.locale)),
            CaseTransform::None => unreachable!("returned above"),
        };
        context.push(ch);
        match mapped {
            Some(mapped) if !mapped.is_empty() => {
                out.extend(mapped.chars().map(|ch| Mapped { ch, start, end }));
            }
            Some(_) => {
                // Dropped by a tailoring: the source joins the previous cluster.
                if let Some(last) = out.last_mut() {
                    let first = last.start;
                    for m in out.iter_mut().rev().take_while(|m| m.start == first) {
                        m.end = end;
                    }
                } else {
                    out.push(Mapped { ch, start, end });
                }
            }
            None => out.push(Mapped { ch, start, end }),
        }
    }
    Some(out)
}

fn is_mark(ch: char) -> bool {
    matches!(ch,
        '\u{300}'..='\u{36f}'
        | '\u{483}'..='\u{489}'
        | '\u{591}'..='\u{5bd}'
        | '\u{610}'..='\u{61a}'
        | '\u{64b}'..='\u{65f}'
        | '\u{1ab0}'..='\u{1aff}'
        | '\u{1dc0}'..='\u{1dff}'
        | '\u{20d0}'..='\u{20ff}'
        | '\u{fe20}'..='\u{fe2f}')
}

fn is_ideographic(ch: char) -> bool {
    matches!(ch,
        '\u{3040}'..='\u{30ff}'
        | '\u{31f0}'..='\u{31ff}'
        | '\u{3400}'..='\u{4dbf}'
        | '\u{4e00}'..='\u{9fff}'
        | '\u{f900}'..='\u{faff}'
        | '\u{ff66}'..='\u{ff9f}'
        | '\u{20000}'..='\u{3ffff}')
}

/// A character that continues a word: a letter or digit (not an ideograph, which
/// is a word of its own) or the connector `_`.
fn is_word_char(ch: char) -> bool {
    (ch.is_alphanumeric() && !is_ideographic(ch)) || ch == '_'
}

/// Punctuation that joins two letters into one word for Chrome's capitalize
/// (apostrophes, middle dots, Hebrew geresh/gershayim, the Arabic decimal mark).
fn is_mid_word(ch: char) -> bool {
    matches!(
        ch,
        '\'' | '\u{2018}' | '\u{2019}' | '\u{b7}' | '\u{5f3}' | '\u{5f4}' | '\u{66b}'
    )
}

/// `capitalize`: the single-character titlecase of a word's first letter.
fn capitalize(ch: char, in_word: bool) -> Option<String> {
    if in_word || !ch.is_alphanumeric() {
        return None;
    }
    let title = match ch {
        '\u{1c4}'..='\u{1c6}' => '\u{1c5}',
        '\u{1c7}'..='\u{1c9}' => '\u{1c8}',
        '\u{1ca}'..='\u{1cc}' => '\u{1cb}',
        '\u{1f1}'..='\u{1f3}' => '\u{1f2}',
        _ => {
            let mut upper = ch.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(single), None) => single,
                _ => return None,
            }
        }
    };
    Some(title.to_string())
}

fn is_soft_dotted(ch: char) -> bool {
    matches!(ch, 'i' | 'j' | '\u{12f}' | '\u{268}')
}

fn upper(ch: char, prev: Option<char>, locale: CaseLocale) -> String {
    match (locale, ch) {
        (CaseLocale::Turkic, 'i') => return "\u{130}".into(),
        (CaseLocale::Lithuanian, '\u{307}') if prev.is_some_and(is_soft_dotted) => {
            return String::new();
        }
        (CaseLocale::Greek, _) => return greek_upper(ch, prev),
        _ => {}
    }
    ch.to_uppercase().collect()
}

/// Greek uppercase (`el`): accents and breathings go, a dialytika stays.
fn greek_upper(ch: char, prev: Option<char>) -> String {
    let greek_base =
        prev.is_some_and(|p| matches!(p, '\u{370}'..='\u{3ff}' | '\u{1f00}'..='\u{1fff}'));
    match ch {
        '\u{300}' | '\u{301}' | '\u{342}' | '\u{313}' | '\u{314}' | '\u{345}' if greek_base => {
            String::new()
        }
        '\u{344}' if greek_base => "\u{308}".into(),
        'ά' | 'Ά' => "Α".into(),
        'έ' | 'Έ' => "Ε".into(),
        'ή' | 'Ή' => "Η".into(),
        'ί' | 'Ί' => "Ι".into(),
        'ό' | 'Ό' => "Ο".into(),
        'ύ' | 'Ύ' => "Υ".into(),
        'ώ' | 'Ώ' => "Ω".into(),
        'ΐ' | 'ϊ' => "Ϊ".into(),
        'ΰ' | 'ϋ' => "Ϋ".into(),
        _ => ch.to_uppercase().collect(),
    }
}

/// Whether a combining mark of the "above" class follows `index`.
fn more_above(chars: &[(usize, char)], index: usize) -> bool {
    chars[index + 1..]
        .iter()
        .map(|c| c.1)
        .take_while(|&c| is_mark(c))
        .any(|c| matches!(c, '\u{300}'..='\u{314}' | '\u{33d}'..='\u{344}' | '\u{346}'))
}

fn lower(chars: &[(usize, char)], index: usize, locale: CaseLocale) -> String {
    let ch = chars[index].1;
    let prev = index.checked_sub(1).map(|i| chars[i].1);
    let next = chars.get(index + 1).map(|c| c.1);
    let mapped: &str = match (locale, ch) {
        (CaseLocale::Turkic, 'I') if next == Some('\u{307}') => "i",
        (CaseLocale::Turkic, 'I') => "\u{131}",
        (CaseLocale::Turkic, '\u{130}') => "i",
        (CaseLocale::Turkic, '\u{307}') if prev == Some('I') => "",
        (CaseLocale::Lithuanian, 'I') if more_above(chars, index) => "i\u{307}",
        (CaseLocale::Lithuanian, 'J') if more_above(chars, index) => "j\u{307}",
        (CaseLocale::Lithuanian, '\u{12e}') if more_above(chars, index) => "\u{12f}\u{307}",
        (CaseLocale::Lithuanian, '\u{cc}') => "i\u{307}\u{300}",
        (CaseLocale::Lithuanian, '\u{cd}') => "i\u{307}\u{301}",
        (CaseLocale::Lithuanian, '\u{128}') => "i\u{307}\u{303}",
        (_, 'Σ') => return final_sigma(chars, index).into(),
        _ => return ch.to_lowercase().collect(),
    };
    mapped.into()
}

/// `Σ` lowercases to `ς` at the end of a word: a cased letter before it and none
/// after it (combining marks are skipped both ways).
fn final_sigma(chars: &[(usize, char)], index: usize) -> &'static str {
    let cased = |c: char| c.is_lowercase() || c.is_uppercase();
    let before = chars[..index]
        .iter()
        .rev()
        .map(|c| c.1)
        .find(|&c| !is_mark(c))
        .is_some_and(cased);
    let after = chars[index + 1..]
        .iter()
        .map(|c| c.1)
        .find(|&c| !is_mark(c))
        .is_some_and(cased);
    if before && !after {
        "ς"
    } else {
        "σ"
    }
}
