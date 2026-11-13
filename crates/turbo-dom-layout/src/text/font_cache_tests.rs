use super::*;

fn face() -> FontFace {
    FontFace::from_bytes(
        include_bytes!("../../assets/fonts/Go-Regular.ttf").to_vec(),
        "Go",
        400,
        false,
    )
    .unwrap()
}

#[test]
fn frequently_used_shapes_survive_unique_run_churn() {
    let font = face();
    let alias = font.with_family("alias");
    let expected = font.shape("retained glyph output");
    for index in 0..512 {
        assert_eq!(alias.shape("retained glyph output"), expected);
        let _ = alias.shape(&format!("unique actual run {index}"));
    }
    assert_eq!(font.cached_shape("retained glyph output"), Some(expected));
    let cache = font.shape_cache.lock().unwrap();
    assert_eq!(cache.entries.len(), SHAPE_CACHE_ENTRIES);
    assert!(cache.bytes <= SHAPE_CACHE_BYTES);
}

#[test]
fn utf8_and_original_utf16_share_one_recency_and_entry_budget() {
    let mut cache = ShapeCache::default();
    cache.insert_units(&[0xd800], &[]);
    for index in 0..SHAPE_CACHE_ENTRIES - 1 {
        cache.insert(&format!("entry {index}"), &[]);
    }
    assert_eq!(cache.get_units(&[0xd800]), Some(vec![]));
    cache.insert("new entry", &[]);
    assert_eq!(cache.get("entry 0"), None);
    assert_eq!(cache.get_units(&[0xd800]), Some(vec![]));
    assert_eq!(
        cache.entries.len() + cache.units_entries.len(),
        SHAPE_CACHE_ENTRIES
    );
    assert_eq!(
        cache.bytes,
        cache
            .entries
            .values()
            .chain(cache.units_entries.values())
            .map(|e| e.bytes)
            .sum::<usize>()
    );
}

#[test]
fn byte_pressure_evicts_only_the_oldest_payload_across_key_types() {
    let mut cache = ShapeCache::default();
    let text = "a".repeat(SHAPE_CACHE_BYTES / 2);
    let units = vec![0x61; SHAPE_CACHE_BYTES / 4];
    cache.insert(&text, &[]);
    cache.insert_units(&units, &[]);
    assert_eq!(cache.bytes, SHAPE_CACHE_BYTES);
    cache.insert("new", &[]);
    assert_eq!(cache.get(&text), None);
    assert_eq!(cache.get_units(&units), Some(vec![]));
    assert_eq!(cache.bytes, SHAPE_CACHE_BYTES / 2 + 3);
}

#[test]
fn recency_counter_rollover_releases_entries_without_overflow() {
    let mut cache = ShapeCache::default();
    cache.insert("old", &[]);
    cache.insert_units(&[0xd800], &[]);
    cache.clock = u64::MAX;
    assert_eq!(cache.get("old"), None);
    assert_eq!(cache.bytes, 0);
    assert_eq!(cache.get_units(&[0xd800]), None);
    cache.insert("fresh", &[]);
    assert_eq!(cache.get("fresh"), Some(vec![]));
    assert_eq!(cache.bytes, 5);
}

#[test]
fn direction_specific_shapes_keep_their_glyphs_under_cache_pressure() {
    let font = face();
    let ltr = font.shape_in("123 abc", false);
    let rtl = font.shape_in("123 abc", true);
    assert_ne!(ltr, rtl);
    for index in 0..256 {
        assert_eq!(font.shape_in("123 abc", false), ltr);
        assert_eq!(font.shape_in("123 abc", true), rtl);
        let _ = font.shape(&format!("row {index}"));
    }
}

#[test]
fn duplicate_utf16_results_keep_first_payload_and_account_it_once() {
    let mut cache = ShapeCache::default();
    cache.insert_units(&[0xd800], &[]);
    let bytes = cache.bytes;
    let glyphs = face().shape("larger competing result");
    cache.insert_units(&[0xd800], &glyphs);
    assert_eq!(cache.bytes, bytes);
    assert_eq!(cache.units_entries.len(), 1);
    assert_eq!(cache.get_units(&[0xd800]), Some(vec![]));
    for index in 0..256 {
        cache.insert(&format!("row {index}"), &glyphs);
    }
    assert_eq!(
        cache.bytes,
        cache
            .entries
            .values()
            .chain(cache.units_entries.values())
            .map(|entry| entry.bytes)
            .sum::<usize>()
    );
    assert!(cache.bytes <= SHAPE_CACHE_BYTES);
}

#[test]
fn font_clones_share_family_storage_while_aliases_keep_their_own_name() {
    let font = face();
    let cloned = font.clone();
    assert!(Arc::ptr_eq(&font.family, &cloned.family));
    assert_eq!(font.family(), "Go");
    let alias = cloned.with_family("other family");
    assert_eq!(alias.family(), "other family");
    assert_eq!(font.family(), "Go");
    assert!(Arc::ptr_eq(&font.faces, &alias.faces));
    assert!(Arc::ptr_eq(&font.shape_cache, &alias.shape_cache));
}

fn assert_raw_mark_cannot_poison_forced(text: &str, rtl: bool, mark: char) {
    assert_ne!(guessed_rtl(text), rtl);
    let raw = format!("{mark}{text}");
    let raw_expected = face().shape(&raw);
    let forced_expected = face().shape_in(text, rtl);
    assert_ne!(raw_expected, forced_expected);
    let shared = face();
    assert_eq!(shared.shape(&raw), raw_expected);
    assert_eq!(shared.shape_in(text, rtl), forced_expected);
    assert_eq!(shared.shape(&raw), raw_expected);
}

fn assert_forced_cannot_poison_raw_mark(text: &str, rtl: bool, mark: char) {
    let raw = format!("{mark}{text}");
    let raw_expected = face().shape(&raw);
    let forced_expected = face().shape_in(text, rtl);
    assert_ne!(raw_expected, forced_expected);
    let shared = face();
    assert_eq!(shared.shape_in(text, rtl), forced_expected);
    assert_eq!(shared.shape(&raw), raw_expected);
    assert_eq!(shared.shape_in(text, rtl), forced_expected);
}

#[test]
fn raw_rtl_mark_cannot_poison_forced_rtl_shape() {
    assert_raw_mark_cannot_poison_forced("abc", true, '\u{200f}');
}

#[test]
fn raw_ltr_mark_cannot_poison_forced_ltr_shape() {
    assert_raw_mark_cannot_poison_forced("אב", false, '\u{200e}');
}

#[test]
fn forced_rtl_shape_cannot_poison_raw_rtl_mark_text() {
    assert_forced_cannot_poison_raw_mark("abc", true, '\u{200f}');
}

#[test]
fn forced_ltr_shape_cannot_poison_raw_ltr_mark_text() {
    assert_forced_cannot_poison_raw_mark("אב", false, '\u{200e}');
}

#[test]
fn forced_shapes_share_the_entry_budget_and_global_recency() {
    let mut cache = ShapeCache::default();
    cache.insert_forced("\u{200f}abc", &[]);
    cache.insert("recent raw", &[]);
    cache.insert_units(&[0xd800], &[]);
    for index in 0..SHAPE_CACHE_ENTRIES - 3 {
        cache.insert(&format!("entry {index}"), &[]);
    }
    assert_eq!(cache.get("recent raw"), Some(vec![]));
    cache.insert("new raw", &[]);
    assert_eq!(cache.get_forced("\u{200f}abc"), None);
    assert_eq!(cache.get_units(&[0xd800]), Some(vec![]));
    assert_eq!(cache.get("recent raw"), Some(vec![]));
    assert_eq!(
        cache.entries.len() + cache.forced_entries.len() + cache.units_entries.len(),
        SHAPE_CACHE_ENTRIES
    );
}

#[test]
fn forced_shapes_share_the_byte_budget_without_flushing_other_domains() {
    let mut cache = ShapeCache::default();
    let forced = "f".repeat(SHAPE_CACHE_BYTES / 2);
    let units = vec![0x61; SHAPE_CACHE_BYTES / 8];
    let raw = "r".repeat(SHAPE_CACHE_BYTES / 4);
    cache.insert_forced(&forced, &[]);
    cache.insert_units(&units, &[]);
    cache.insert(&raw, &[]);
    assert_eq!(cache.bytes, SHAPE_CACHE_BYTES);
    assert_eq!(cache.get_forced(&forced), Some(vec![]));
    cache.insert("x", &[]);
    assert_eq!(cache.get_units(&units), None);
    assert_eq!(cache.get_forced(&forced), Some(vec![]));
    assert_eq!(cache.get(&raw), Some(vec![]));
    assert_eq!(cache.bytes, SHAPE_CACHE_BYTES * 3 / 4 + 1);
}

#[test]
fn forced_shape_duplicates_and_oversize_do_not_corrupt_payload_accounting() {
    let mut cache = ShapeCache::default();
    cache.insert_forced("\u{200f}abc", &[]);
    let bytes = cache.bytes;
    cache.insert_forced("\u{200f}abc", &face().shape("larger result"));
    assert_eq!(cache.get_forced("\u{200f}abc"), Some(vec![]));
    assert_eq!(cache.bytes, bytes);
    cache.insert_forced(&"x".repeat(SHAPE_CACHE_BYTES + 1), &[]);
    assert_eq!(cache.bytes, bytes);
    assert_eq!(cache.forced_entries.len(), 1);
    cache.clock = u64::MAX;
    assert_eq!(cache.get_forced("\u{200f}abc"), None);
    assert_eq!(cache.bytes, 0);
    assert!(cache.entries.is_empty());
    assert!(cache.forced_entries.is_empty());
    assert!(cache.units_entries.is_empty());
}
