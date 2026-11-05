//! `hyphens: auto` dictionary hyphenation (CSS Text 3 §5.4, miaskiewicz/turbo-dom#213).
//!
//! English (`lang` `en` or `en-*`, except `en-GB`, whose dictionary differs) breaks at
//! the en-US Knuth-Liang points (the TeX `hyph-en-us` patterns Chrome's own Minikin
//! dictionaries are built from, embedded by the `hyphenation` crate under the default-on
//! `hyphenation` cargo feature, about 90 KB). Like Blink: only words of letters at least
//! `hyphenate-limit-chars` long (default `auto` = 5 / 2 / 2: word, before and after the
//! break) hyphenate, a capitalized word does not (English), and a word with a soft
//! hyphen breaks only there. Other languages, and builds without the feature, never
//! hyphenate automatically.

/// A run's `hyphens: auto` settings in a supported language: the
/// `hyphenate-limit-chars` minimum word, prefix and suffix lengths in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoHyphens {
    pub min_word: usize,
    pub min_prefix: usize,
    pub min_suffix: usize,
}

impl AutoHyphens {
    /// From the computed `hyphens`, content language (`-webkit-locale`, from `lang`)
    /// and `hyphenate-limit-chars`: `None` unless `hyphens: auto` in a supported
    /// language.
    #[must_use]
    pub fn from_computed(
        hyphens: Option<&str>,
        locale: Option<&str>,
        limit_chars: Option<&str>,
    ) -> Option<Self> {
        if hyphens.map(str::trim) != Some("auto") || !supported(locale?) {
            return None;
        }
        // `auto` (or a value that is not an integer) keeps the default; a missing
        // third value repeats the second (CSS Text 4 §6.3).
        let mut values = limit_chars.unwrap_or("auto").split_whitespace();
        let mut next = |default: usize| {
            values
                .next()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(default)
        };
        let min_word = next(5);
        let min_prefix = next(2);
        let min_suffix = next(min_prefix);
        Some(Self {
            min_word,
            min_prefix,
            min_suffix,
        })
    }
}

/// `en` and its regional variants, but not `en-GB` (British hyphenation differs).
fn supported(locale: &str) -> bool {
    let locale = locale.trim().to_ascii_lowercase().replace('_', "-");
    let mut subtags = locale.split('-');
    cfg!(feature = "hyphenation") && subtags.next() == Some("en") && subtags.next() != Some("gb")
}

/// The dictionary break points of `word` (lowercase letters), as character indices a
/// break may precede.
#[cfg(feature = "hyphenation")]
fn dictionary_breaks(word: &str) -> Vec<usize> {
    use hyphenation::{Hyphenator, Language, Load, Standard};
    static DICTIONARY: std::sync::OnceLock<Standard> = std::sync::OnceLock::new();
    let dictionary = DICTIONARY.get_or_init(|| {
        let mut dictionary = Standard::from_embedded(Language::EnglishUS)
            .expect("the embedded en-US dictionary deserializes");
        // The limits are `hyphenate-limit-chars`, applied by the caller.
        dictionary.minima = (1, 1);
        dictionary
    });
    dictionary
        .hyphenate(word)
        .breaks
        .into_iter()
        .map(|byte| word[..byte].chars().count())
        .collect()
}

#[cfg(not(feature = "hyphenation"))]
fn dictionary_breaks(_: &str) -> Vec<usize> {
    unreachable!("no language is supported without the `hyphenation` feature")
}

/// The break points (character indices into `letters`) of one word of letters under
/// `limits`: none for a capitalized or too-short word, or one whose lowercase form is
/// not one character per letter.
pub(super) fn word_breaks(letters: &[char], limits: AutoHyphens) -> Vec<usize> {
    let count = letters.len();
    if count < limits.min_word || letters.first().is_none_or(|c| c.is_uppercase()) {
        return Vec::new();
    }
    let mut word = String::with_capacity(count);
    for &letter in letters {
        let mut lower = letter.to_lowercase();
        match (lower.next(), lower.next()) {
            (Some(l), None) => word.push(l),
            _ => return Vec::new(),
        }
    }
    dictionary_breaks(&word)
        .into_iter()
        .filter(|&at| at >= limits.min_prefix && count - at >= limits.min_suffix)
        .collect()
}

#[cfg(all(test, feature = "hyphenation"))]
mod tests {
    use super::{word_breaks, AutoHyphens};

    const DEFAULT: AutoHyphens = AutoHyphens {
        min_word: 5,
        min_prefix: 2,
        min_suffix: 2,
    };

    #[test]
    fn hyphens_auto_needs_a_supported_english_locale() {
        let auto = |locale| AutoHyphens::from_computed(Some(" auto "), locale, None);
        assert_eq!(auto(Some("en")), Some(DEFAULT));
        assert_eq!(auto(Some("EN_us")), Some(DEFAULT));
        assert_eq!(auto(Some("en-CA")), Some(DEFAULT));
        assert_eq!(auto(Some("en-GB")), None);
        assert_eq!(auto(Some("de")), None);
        assert_eq!(auto(Some("eng")), None);
        assert_eq!(auto(None), None);
        assert_eq!(
            AutoHyphens::from_computed(Some("manual"), Some("en"), None),
            None
        );
        assert_eq!(AutoHyphens::from_computed(None, Some("en"), None), None);
    }

    #[test]
    fn hyphenate_limit_chars_repeats_the_prefix_for_a_missing_suffix() {
        let limits = |value| AutoHyphens::from_computed(Some("auto"), Some("en"), Some(value));
        let three = |min_word, min_prefix, min_suffix| {
            Some(AutoHyphens {
                min_word,
                min_prefix,
                min_suffix,
            })
        };
        assert_eq!(limits("auto"), three(5, 2, 2));
        assert_eq!(limits("7"), three(7, 2, 2));
        assert_eq!(limits("auto 3"), three(5, 3, 3));
        assert_eq!(limits("6 auto 4"), three(6, 2, 4));
        assert_eq!(limits("x 1 1"), three(5, 1, 1));
    }

    #[test]
    fn word_breaks_apply_the_limits_and_skip_capitalized_words() {
        let breaks = |word: &str, limits| word_breaks(&word.chars().collect::<Vec<_>>(), limits);
        assert_eq!(breaks("typography", DEFAULT), vec![2, 5, 7]);
        assert_eq!(breaks("Typography", DEFAULT), Vec::<usize>::new());
        assert_eq!(breaks("TYPOGRAPHY", DEFAULT), Vec::<usize>::new());
        assert_eq!(breaks("tyPOGRAPHY", DEFAULT), vec![2, 5, 7]);
        let long_prefix = AutoHyphens {
            min_prefix: 4,
            min_suffix: 4,
            ..DEFAULT
        };
        assert_eq!(breaks("typography", long_prefix), vec![5]);
        assert_eq!(breaks("pa", DEFAULT), Vec::<usize>::new());
        let any = AutoHyphens {
            min_word: 0,
            ..DEFAULT
        };
        assert_eq!(breaks("", any), Vec::<usize>::new());
        // U+0130 lowercases to two characters: not hyphenated.
        assert_eq!(breaks("typ\u{130}graphy", DEFAULT), Vec::<usize>::new());
    }

    /// Every word of the per-language control (`tests/fixtures/hyphens/words-chrome156.json`,
    /// `words-probe.js`: Chrome 156 on macOS, default limits) breaks where Chrome does,
    /// except where the macOS system dictionary Chrome uses there differs from the TeX
    /// patterns (Chrome on Linux, Windows and Android hyphenates with those patterns).
    #[cfg(feature = "hyphenation-languages")]
    #[test]
    fn word_breaks_match_chrome_156_but_for_the_macos_dictionary_differences() {
        const MACOS_DICTIONARY: [(&str, &str, &[usize]); 8] = [
            ("en", "international", &[2, 5, 7, 11]),
            ("en", "extraordinary", &[2, 5, 7, 9]),
            ("es", "desarrollo", &[2, 4, 7]),
            ("pl", "odpowiedzialność", &[2, 4, 7, 12]),
            ("pl", "szczególnie", &[5, 8]),
            ("pl", "elektryczność", &[4, 9]),
            ("pl", "chrząszcz", &[]),
            ("cs", "knihovna", &[6]),
        ];
        let control: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/hyphens/words-chrome156.json"
        ))
        .unwrap();
        assert_eq!(control["browser"], "Chrome/156.0.8078.47");
        let mut differences = Vec::new();
        let mut words = 0;
        for (lang, list) in control["result"]["words"].as_object().unwrap() {
            let limits = AutoHyphens::from_computed(Some("auto"), Some(lang), None);
            for (word, chrome) in list.as_object().unwrap() {
                words += 1;
                let chrome: Vec<usize> = chrome
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|at| usize::try_from(at.as_u64().unwrap()).unwrap())
                    .collect();
                let letters: Vec<char> = word.chars().collect();
                let got = limits.map_or_else(Vec::new, |limits| word_breaks(&letters, limits));
                if got != chrome {
                    differences.push((lang.as_str(), word.as_str(), got));
                }
            }
        }
        assert_eq!(words, 330, "every control word");
        let expected: Vec<_> = MACOS_DICTIONARY
            .iter()
            .map(|&(lang, word, tex)| (lang, word, tex.to_vec()))
            .collect();
        assert_eq!(differences, expected);
    }
}
