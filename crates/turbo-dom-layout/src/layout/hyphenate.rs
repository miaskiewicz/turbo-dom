//! `hyphens: auto` dictionary hyphenation (CSS Text 3 §5.4, miaskiewicz/turbo-dom#213,
//! #289).
//!
//! The languages are the ones Chrome 156 on macOS hyphenates ([`HyphenationLanguage`]),
//! broken at the Knuth-Liang points of the TeX `hyph-*` patterns (the ones Chrome's own
//! Minikin dictionaries on Linux, Windows and Android are built from), embedded by the
//! `hyphenation` crate: en-US alone under the default-on `hyphenation` cargo feature
//! (about 90 KB), every language under the default-on `hyphenation-languages` (about
//! 3 MB). Chrome on macOS hyphenates with the system dictionary instead, whose points
//! differ for a few words (en `international`, pl `szczególnie`; the words control
//! lists them). Like Blink: only words of letters at least `hyphenate-limit-chars` long
//! (default `auto` = 5 / 2 / 2: word, before and after the break) hyphenate, a
//! capitalized word does not in English (it does in every other language), and a word
//! with a soft hyphen breaks only there. Other languages, and builds without the
//! dictionary, never hyphenate automatically.

/// A content language `hyphens: auto` hyphenates: the ones Chrome 156 on macOS has a
/// dictionary for, by the primary language subtag (`en-GB` apart from other English).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HyphenationLanguage {
    /// `en` and its regions other than `GB`.
    EnglishUS,
    /// `en-GB`.
    EnglishGB,
    /// `ca`.
    Catalan,
    /// `cs`.
    Czech,
    /// `da`.
    Danish,
    /// `de` (the 1996 orthography).
    German,
    /// `el` (monotonic).
    Greek,
    /// `es`.
    Spanish,
    /// `fi`.
    Finnish,
    /// `fr`.
    French,
    /// `hr`.
    Croatian,
    /// `hu`.
    Hungarian,
    /// `it`.
    Italian,
    /// `nb` and `no`.
    NorwegianBokmal,
    /// `nl`.
    Dutch,
    /// `pl`.
    Polish,
    /// `pt`.
    Portuguese,
    /// `ro`.
    Romanian,
    /// `ru`.
    Russian,
    /// `sk`.
    Slovak,
    /// `sv`.
    Swedish,
    /// `uk`.
    Ukrainian,
}

/// How many [`HyphenationLanguage`]s there are (one cached dictionary each).
#[cfg(feature = "hyphenation")]
const LANGUAGES: usize = 22;
#[cfg(feature = "hyphenation")]
const _: () = assert!(HyphenationLanguage::Ukrainian as usize + 1 == LANGUAGES);

impl HyphenationLanguage {
    /// The language of a BCP 47 `locale` (case-insensitive, `_` accepted for `-`):
    /// `None` when it is not one Chrome hyphenates or this build has no dictionary for
    /// it (`en-US` alone without the `hyphenation-languages` feature, nothing without
    /// `hyphenation`).
    #[must_use]
    pub fn from_locale(locale: &str) -> Option<Self> {
        let locale = locale.trim().to_ascii_lowercase().replace('_', "-");
        let mut subtags = locale.split('-');
        let primary = subtags.next().unwrap_or_default();
        let language = Self::from_subtags(primary, subtags.next())?;
        embedded(language).then_some(language)
    }

    fn from_subtags(primary: &str, region: Option<&str>) -> Option<Self> {
        Some(match (primary, region) {
            ("en", Some("gb")) => Self::EnglishGB,
            ("en", _) => Self::EnglishUS,
            ("ca", _) => Self::Catalan,
            ("cs", _) => Self::Czech,
            ("da", _) => Self::Danish,
            ("de", _) => Self::German,
            ("el", _) => Self::Greek,
            ("es", _) => Self::Spanish,
            ("fi", _) => Self::Finnish,
            ("fr", _) => Self::French,
            ("hr", _) => Self::Croatian,
            ("hu", _) => Self::Hungarian,
            ("it", _) => Self::Italian,
            ("nb" | "no", _) => Self::NorwegianBokmal,
            ("nl", _) => Self::Dutch,
            ("pl", _) => Self::Polish,
            ("pt", _) => Self::Portuguese,
            ("ro", _) => Self::Romanian,
            ("ru", _) => Self::Russian,
            ("sk", _) => Self::Slovak,
            ("sv", _) => Self::Swedish,
            ("uk", _) => Self::Ukrainian,
            _ => return None,
        })
    }

    /// Blink leaves capitalized words alone in English only.
    fn hyphenates_capitalized(self) -> bool {
        !matches!(self, Self::EnglishUS | Self::EnglishGB)
    }

    /// The `hyphenation` crate's patterns for the language.
    #[cfg(feature = "hyphenation")]
    fn patterns(self) -> hyphenation::Language {
        use hyphenation::Language;
        match self {
            Self::EnglishUS => Language::EnglishUS,
            Self::EnglishGB => Language::EnglishGB,
            Self::Catalan => Language::Catalan,
            Self::Czech => Language::Czech,
            Self::Danish => Language::Danish,
            Self::German => Language::German1996,
            Self::Greek => Language::GreekMono,
            Self::Spanish => Language::Spanish,
            Self::Finnish => Language::Finnish,
            Self::French => Language::French,
            Self::Croatian => Language::Croatian,
            Self::Hungarian => Language::Hungarian,
            Self::Italian => Language::Italian,
            Self::NorwegianBokmal => Language::NorwegianBokmal,
            Self::Dutch => Language::Dutch,
            Self::Polish => Language::Polish,
            Self::Portuguese => Language::Portuguese,
            Self::Romanian => Language::Romanian,
            Self::Russian => Language::Russian,
            Self::Slovak => Language::Slovak,
            Self::Swedish => Language::Swedish,
            Self::Ukrainian => Language::Ukrainian,
        }
    }
}

/// Whether this build embeds the language's dictionary: every one under
/// `hyphenation-languages`.
#[cfg(feature = "hyphenation-languages")]
fn embedded(_: HyphenationLanguage) -> bool {
    true
}

/// Whether this build embeds the language's dictionary: en-US alone under
/// `hyphenation`, none without it.
#[cfg(not(feature = "hyphenation-languages"))]
fn embedded(language: HyphenationLanguage) -> bool {
    cfg!(feature = "hyphenation") && language == HyphenationLanguage::EnglishUS
}

/// A run's `hyphens: auto` settings in a supported language: the language and the
/// `hyphenate-limit-chars` minimum word, prefix and suffix lengths in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoHyphens {
    pub language: HyphenationLanguage,
    pub min_word: usize,
    pub min_prefix: usize,
    pub min_suffix: usize,
}

impl AutoHyphens {
    /// From the computed `hyphens`, content language (`-webkit-locale`, from `lang`)
    /// and `hyphenate-limit-chars`: `None` unless `hyphens: auto` in a supported
    /// language ([`HyphenationLanguage::from_locale`]).
    #[must_use]
    pub fn from_computed(
        hyphens: Option<&str>,
        locale: Option<&str>,
        limit_chars: Option<&str>,
    ) -> Option<Self> {
        if hyphens.map(str::trim) != Some("auto") {
            return None;
        }
        let language = HyphenationLanguage::from_locale(locale?)?;
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
            language,
            min_word,
            min_prefix,
            min_suffix,
        })
    }
}

/// The dictionary break points of `word` (lowercase letters) in `language`, as
/// character indices a break may precede.
#[cfg(feature = "hyphenation")]
fn dictionary_breaks(word: &str, language: HyphenationLanguage) -> Vec<usize> {
    use hyphenation::{Hyphenator, Load, Standard};
    static DICTIONARIES: [std::sync::OnceLock<Standard>; LANGUAGES] =
        [const { std::sync::OnceLock::new() }; LANGUAGES];
    let dictionary = DICTIONARIES[language as usize].get_or_init(|| {
        let mut dictionary = Standard::from_embedded(language.patterns())
            .expect("an embedded dictionary deserializes");
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
fn dictionary_breaks(_: &str, _: HyphenationLanguage) -> Vec<usize> {
    unreachable!("no language is supported without the `hyphenation` feature")
}

/// The break points (character indices into `letters`) of one word of letters under
/// `limits`: none for a too-short word, a capitalized one in English, or one whose
/// lowercase form is not one character per letter.
pub(super) fn word_breaks(letters: &[char], limits: AutoHyphens) -> Vec<usize> {
    let count = letters.len();
    let Some(first) = letters.first() else {
        return Vec::new();
    };
    if count < limits.min_word
        || (first.is_uppercase() && !limits.language.hyphenates_capitalized())
    {
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
    dictionary_breaks(&word, limits.language)
        .into_iter()
        .filter(|&at| at >= limits.min_prefix && count - at >= limits.min_suffix)
        .collect()
}

#[cfg(all(test, feature = "hyphenation"))]
mod tests {
    use super::{word_breaks, AutoHyphens, HyphenationLanguage};

    const DEFAULT: AutoHyphens = AutoHyphens {
        language: HyphenationLanguage::EnglishUS,
        min_word: 5,
        min_prefix: 2,
        min_suffix: 2,
    };

    #[test]
    fn hyphens_auto_needs_a_supported_locale() {
        let auto = |locale| AutoHyphens::from_computed(Some(" auto "), locale, None);
        assert_eq!(auto(Some("en")), Some(DEFAULT));
        assert_eq!(auto(Some("EN_us")), Some(DEFAULT));
        assert_eq!(auto(Some("en-CA")), Some(DEFAULT));
        assert_eq!(auto(Some("eng")), None);
        assert_eq!(auto(Some("")), None);
        assert_eq!(auto(None), None);
        assert_eq!(
            AutoHyphens::from_computed(Some("manual"), Some("en"), None),
            None
        );
        assert_eq!(AutoHyphens::from_computed(None, Some("en"), None), None);
        // Languages Chrome 156 on macOS does not hyphenate (turbo-dom#289).
        for locale in ["nn", "tr", "sl", "sr", "bg", "lt", "la", "zh", "xx"] {
            assert_eq!(auto(Some(locale)), None, "{locale}");
        }
    }

    /// Without `hyphenation-languages` only the en-US dictionary is embedded.
    #[cfg(not(feature = "hyphenation-languages"))]
    #[test]
    fn hyphens_auto_is_english_only_without_the_languages_feature() {
        assert_eq!(HyphenationLanguage::from_locale("en-GB"), None);
        assert_eq!(HyphenationLanguage::from_locale("de"), None);
        assert_eq!(
            HyphenationLanguage::from_locale("en"),
            Some(HyphenationLanguage::EnglishUS)
        );
    }

    #[cfg(feature = "hyphenation-languages")]
    #[test]
    fn hyphenation_languages_follow_the_primary_subtag() {
        use HyphenationLanguage as L;
        for (locale, language) in [
            ("en-GB", L::EnglishGB),
            ("en_gb", L::EnglishGB),
            ("en-AU", L::EnglishUS),
            ("ca", L::Catalan),
            ("cs-CZ", L::Czech),
            ("da", L::Danish),
            ("de-CH", L::German),
            ("DE", L::German),
            ("el", L::Greek),
            ("es-MX", L::Spanish),
            ("fi", L::Finnish),
            ("fr-CA", L::French),
            ("hr", L::Croatian),
            ("hu", L::Hungarian),
            ("it", L::Italian),
            ("nb", L::NorwegianBokmal),
            ("no-NO", L::NorwegianBokmal),
            ("nl-BE", L::Dutch),
            ("pl", L::Polish),
            ("pt-BR", L::Portuguese),
            ("ro", L::Romanian),
            ("ru", L::Russian),
            ("sk", L::Slovak),
            ("sv", L::Swedish),
            ("uk", L::Ukrainian),
        ] {
            assert_eq!(
                HyphenationLanguage::from_locale(locale),
                Some(language),
                "{locale}"
            );
        }
    }

    #[cfg(feature = "hyphenation-languages")]
    #[test]
    fn capitalized_words_hyphenate_outside_english() {
        let breaks = |word: &str, language| {
            let limits = AutoHyphens {
                language,
                ..DEFAULT
            };
            word_breaks(&word.chars().collect::<Vec<_>>(), limits)
        };
        assert_eq!(
            breaks("Silbentrennung", HyphenationLanguage::German),
            vec![3, 6, 10]
        );
        assert_eq!(
            breaks("SILBENTRENNUNG", HyphenationLanguage::German),
            vec![3, 6, 10]
        );
        assert_eq!(
            breaks("International", HyphenationLanguage::EnglishGB),
            Vec::<usize>::new()
        );
        assert_eq!(
            breaks("international", HyphenationLanguage::EnglishGB),
            vec![2, 5, 7, 11]
        );
    }

    #[test]
    fn hyphenate_limit_chars_repeats_the_prefix_for_a_missing_suffix() {
        let limits = |value| AutoHyphens::from_computed(Some("auto"), Some("en"), Some(value));
        let three = |min_word, min_prefix, min_suffix| {
            Some(AutoHyphens {
                language: HyphenationLanguage::EnglishUS,
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
