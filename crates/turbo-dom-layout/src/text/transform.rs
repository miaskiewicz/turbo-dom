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
        let mapped = match transform.case {
            CaseTransform::Capitalize => capitalize(ch, context.in_word),
            CaseTransform::Uppercase => Some(upper(&chars, index, transform.locale)),
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
                }
            }
            None => out.push(Mapped { ch, start, end }),
        }
    }
    Some(out)
}

pub(crate) fn is_mark(ch: char) -> bool {
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

fn upper(chars: &[(usize, char)], index: usize, locale: CaseLocale) -> String {
    let ch = chars[index].1;
    let prev = index.checked_sub(1).map(|i| chars[i].1);
    match (locale, ch) {
        (CaseLocale::Turkic, 'i') => return "\u{130}".into(),
        (CaseLocale::Lithuanian, '\u{307}') if prev.is_some_and(is_soft_dotted) => {
            return String::new();
        }
        (CaseLocale::Greek, _) => return greek_upper(chars, index),
        _ => {}
    }
    ch.to_uppercase().collect()
}

/// Greek uppercase (`el`), as ICU's `GreekUpper` (Chrome 156): accents and
/// breathings go and a dialytika stays; `ι`/`υ` after an accented vowel is no
/// diphthong and gains a dialytika; a lone `ή` (the disjunctive "or") keeps it.
fn greek_upper(chars: &[(usize, char)], index: usize) -> String {
    let ch = chars[index].1;
    let prev = index.checked_sub(1).map(|i| chars[i].1);
    let greek_base = prev.is_some_and(is_greek);
    let dropped = matches!(
        ch,
        '\u{300}' | '\u{301}' | '\u{342}' | '\u{313}' | '\u{314}' | '\u{345}'
    );
    if let Some(letter) = polytonic(ch) {
        return polytonic_upper(letter, chars, index);
    }
    match ch {
        _ if greek_base && dropped => String::new(),
        '\u{344}' if greek_base => "\u{308}".into(),
        'ή' | 'Ή' if !cased_around(chars, index) => "\u{389}".into(),
        'ι' | 'ί' | 'Ι' | 'Ί' | 'υ' | 'ύ' | 'Υ' | 'Ύ' => {
            match after_accented_vowel(chars, index) {
                Some(true) => if matches!(ch, 'ι' | 'ί' | 'Ι' | 'Ί') {
                    "Ϊ"
                } else {
                    "Ϋ"
                }
                .into(),
                Some(false) => format!("{}\u{308}", greek_plain_upper(ch)),
                None => greek_plain_upper(ch),
            }
        }
        _ => greek_plain_upper(ch),
    }
}

/// A precomposed letter of the Greek Extended block (U+1F00–U+1FFF), decomposed:
/// its base letter (lowercase), whether it carries an accent or breathing, a
/// dialytika, and a ypogegrammeni / prosgegrammeni (turbo-dom#214).
#[derive(Clone, Copy)]
struct Polytonic {
    base: char,
    accent: bool,
    dialytika: bool,
    iota: bool,
}

/// The [`Polytonic`] decomposition of `ch`; `None` outside the block's letters.
fn polytonic(ch: char) -> Option<Polytonic> {
    let code = u32::from(ch);
    let row = |base: char, accent: bool, dialytika: bool, iota: bool| {
        Some(Polytonic {
            base,
            accent,
            dialytika,
            iota,
        })
    };
    // U+1F00–U+1F6F: eight breathing/accent variants per vowel, lower then upper.
    const ROWS: [char; 7] = ['α', 'ε', 'η', 'ι', 'ο', 'υ', 'ω'];
    match code {
        0x1f00..=0x1f6f => {
            let base = ROWS[((code - 0x1f00) / 0x10) as usize];
            let assigned = match base {
                'ε' | 'ο' => code & 0x7 <= 5,
                'υ' => code & 0x8 == 0 || code & 0x1 == 1,
                _ => true,
            };
            if assigned {
                row(base, true, false, false)
            } else {
                None
            }
        }
        // Varia / oxia pairs.
        0x1f70..=0x1f7d => row(ROWS[((code - 0x1f70) / 2) as usize], true, false, false),
        // Breathing (+ accent) with ypogegrammeni: α, η, ω.
        0x1f80..=0x1faf => {
            let base = ['α', 'η', 'ω'][((code - 0x1f80) / 0x10) as usize];
            row(base, true, false, true)
        }
        _ => polytonic_rest(code),
    }
}

/// U+1FB0–U+1FFF of [`polytonic`]: vrachy / macron, accents with ypogegrammeni,
/// perispomeni, dialytika with accents and the rough-breathing rho.
fn polytonic_rest(code: u32) -> Option<Polytonic> {
    let (base, accent, dialytika, iota) = match code {
        0x1fb0 | 0x1fb1 | 0x1fb8 | 0x1fb9 => ('α', false, false, false),
        0x1fb2 | 0x1fb4 | 0x1fb7 => ('α', true, false, true),
        0x1fb3 | 0x1fbc => ('α', false, false, true),
        0x1fb6 | 0x1fba | 0x1fbb => ('α', true, false, false),
        0x1fc2 | 0x1fc4 | 0x1fc7 => ('η', true, false, true),
        0x1fc3 | 0x1fcc => ('η', false, false, true),
        0x1fc6 | 0x1fca | 0x1fcb => ('η', true, false, false),
        0x1fc8 | 0x1fc9 => ('ε', true, false, false),
        0x1fd0 | 0x1fd1 | 0x1fd8 | 0x1fd9 => ('ι', false, false, false),
        0x1fd2 | 0x1fd3 | 0x1fd7 => ('ι', true, true, false),
        0x1fd6 | 0x1fda | 0x1fdb => ('ι', true, false, false),
        0x1fe0 | 0x1fe1 | 0x1fe8 | 0x1fe9 => ('υ', false, false, false),
        0x1fe2 | 0x1fe3 | 0x1fe7 => ('υ', true, true, false),
        0x1fe4 | 0x1fe5 | 0x1fec => ('ρ', true, false, false),
        0x1fe6 | 0x1fea | 0x1feb => ('υ', true, false, false),
        0x1ff2 | 0x1ff4 | 0x1ff7 => ('ω', true, false, true),
        0x1ff3 | 0x1ffc => ('ω', false, false, true),
        0x1ff6 | 0x1ffa | 0x1ffb => ('ω', true, false, false),
        0x1ff8 | 0x1ff9 => ('ο', true, false, false),
        _ => return None,
    };
    Some(Polytonic {
        base,
        accent,
        dialytika,
        iota,
    })
}

/// A Greek Extended letter's `el` uppercase: the plain capital, a dialytika kept
/// (or gained after an accented vowel), and a ypogegrammeni as a capital iota.
fn polytonic_upper(letter: Polytonic, chars: &[(usize, char)], index: usize) -> String {
    let gains_dialytika = !letter.dialytika
        && matches!(letter.base, 'ι' | 'υ')
        && after_accented_vowel(chars, index).is_some();
    let mut out = match (letter.base, letter.dialytika || gains_dialytika) {
        ('ι', true) => "Ϊ".to_string(),
        ('υ', true) => "Ϋ".to_string(),
        (base, _) => base.to_uppercase().collect(),
    };
    if letter.iota {
        out.push('Ι');
    }
    out
}

fn is_greek(ch: char) -> bool {
    matches!(ch, '\u{370}'..='\u{3ff}' | '\u{1f00}'..='\u{1fff}')
}

/// A Greek letter's uppercase without accents.
fn greek_plain_upper(ch: char) -> String {
    match ch {
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

/// The letter before `index` (combining marks skipped) and the marks after it.
fn previous_letter(chars: &[(usize, char)], index: usize) -> Option<(char, &[(usize, char)])> {
    let letter = chars[..index].iter().rposition(|c| !is_mark(c.1))?;
    Some((chars[letter].1, &chars[letter + 1..index]))
}

/// Whether the previous letter is a Greek vowel with an accent and no dialytika:
/// `Some(true)` for a precomposed accent, `Some(false)` for a combining one.
fn after_accented_vowel(chars: &[(usize, char)], index: usize) -> Option<bool> {
    let (letter, marks) = previous_letter(chars, index)?;
    let at = index - marks.len() - 1;
    let precomposed = matches!(
        letter,
        'ά' | 'έ' | 'ή' | 'ί' | 'ό' | 'ύ' | 'ώ' | 'Ά' | 'Έ' | 'Ή' | 'Ί' | 'Ό' | 'Ύ' | 'Ώ'
    ) || polytonic(letter).is_some_and(|p| p.accent && !p.dialytika);
    let combining = matches!(
        letter,
        'α' | 'ε' | 'η' | 'ι' | 'ο' | 'υ' | 'ω' | 'Α' | 'Ε' | 'Η' | 'Ι' | 'Ο' | 'Υ' | 'Ω'
    ) && marks
        .iter()
        .any(|m| matches!(m.1, '\u{300}' | '\u{301}' | '\u{342}'));
    // An ι/υ that itself gained a dialytika ends the context.
    let iota_upsilon = matches!(letter, 'ι' | 'ί' | 'Ι' | 'Ί' | 'υ' | 'ύ' | 'Υ' | 'Ύ')
        || polytonic(letter).is_some_and(|p| matches!(p.base, 'ι' | 'υ'));
    if !(precomposed || combining) || (iota_upsilon && after_accented_vowel(chars, at).is_some()) {
        return None;
    }
    Some(precomposed)
}

/// Whether a cased letter sits right before or after `index` (marks skipped).
fn cased_around(chars: &[(usize, char)], index: usize) -> bool {
    let cased = |c: char| c.is_lowercase() || c.is_uppercase();
    previous_letter(chars, index).is_some_and(|(c, _)| cased(c))
        || chars[index + 1..]
            .iter()
            .map(|c| c.1)
            .find(|&c| !is_mark(c))
            .is_some_and(cased)
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
