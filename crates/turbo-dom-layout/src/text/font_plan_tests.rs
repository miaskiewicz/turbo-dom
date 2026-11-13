use super::*;

fn face() -> FontFace {
    FontFace::collection_fixture_faces()[0].clone()
}

fn control(face: &FontFace, text: &str, rtl: Option<bool>) -> Vec<ShapedGlyph> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    if let Some(rtl) = rtl {
        buffer.guess_segment_properties();
        buffer.set_direction(if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        });
    }
    collect_glyphs(&rustybuzz::shape(
        &face.faces.borrow_dependent().rb,
        &[],
        buffer,
    ))
}

#[test]
fn plans_preserve_fresh_shaping_across_scripts_clusters_and_directions() {
    let f = face();
    let inputs = [
        "Office AV fi",
        "אבג 123",
        "مرحبا",
        "漢字かな",
        "e\u{301}",
        "👩\u{200d}💻",
        "1234.50",
        "\u{fffd}",
        "",
    ];
    for text in inputs {
        assert_eq!(f.shape(text), control(&f, text, None), "{text:?}");
        for rtl in [false, true] {
            assert_eq!(f.shape_in(text, rtl), control(&f, text, Some(rtl)));
        }
    }
}

#[test]
fn face_aliases_share_plans_but_collection_indexes_do_not() {
    let [first, second] = FontFace::collection_fixture_faces();
    let alias = first.with_family("other family");
    assert!(Arc::ptr_eq(&first.shape_cache, &alias.shape_cache));
    assert!(!Arc::ptr_eq(&first.shape_cache, &second.shape_cache));
    assert_eq!(first.shape("Office"), control(&first, "Office", None));
    assert_eq!(second.shape("Office"), control(&second, "Office", None));
}

fn language_buffer(language: usize) -> UnicodeBuffer {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str("hello");
    buffer.guess_segment_properties();
    buffer.set_language(format!("en-x-{language}").parse().unwrap());
    buffer
}

fn retained_language(f: &FontFace, language: usize) -> bool {
    let key = PlanKey::of(&language_buffer(language));
    f.shape_cache
        .lock()
        .unwrap()
        .plans
        .entries
        .iter()
        .any(|entry| entry.key == key)
}

#[test]
fn plan_retention_is_bounded_and_uses_full_language_key() {
    let f = face();
    for language in 0..PLAN_CACHE_ENTRIES {
        let _ = f.shape_buffer(language_buffer(language));
    }
    let _ = f.shape_buffer(language_buffer(0));
    let _ = f.shape_buffer(language_buffer(PLAN_CACHE_ENTRIES));
    assert!(retained_language(&f, 0));
    assert!(!retained_language(&f, 1));
    assert!(retained_language(&f, PLAN_CACHE_ENTRIES));
    assert_eq!(
        f.shape_cache.lock().unwrap().plans.entries.len(),
        PLAN_CACHE_ENTRIES
    );
    assert_eq!(f.shape_cache.lock().unwrap().bytes, 0);
}

#[test]
fn duplicate_plan_insertion_keeps_first_writer_without_growing() {
    let f = face();
    let buffer = language_buffer(0);
    let key = PlanKey::of(&buffer);
    let first = f.create_plan(key.clone());
    let duplicate = f.create_plan(key);
    assert!(Arc::ptr_eq(&first, &duplicate));
    assert_eq!(f.shape_cache.lock().unwrap().plans.entries.len(), 1);
}

#[test]
fn utf16_units_keep_replacement_glyphs_and_source_clusters() {
    let f = face();
    let original = [65, 0xd800, 66, 0xdc00];
    let first = f.shape_original_utf16(&original);
    for i in 0..256 {
        let _ = f.shape(&format!("Item {i}"));
    }
    let second = f.shape_original_utf16(&original);
    assert_eq!(first, second);
    assert_eq!(first, face().shape_original_utf16(&original));
}

#[test]
fn concurrent_shaping_keeps_face_plan_and_glyph_results_isolated() {
    let f = face();
    let expected = control(&f, "Office 123", None);
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let cloned = f.clone();
            std::thread::spawn(move || cloned.shape("Office 123"))
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), expected);
    }
    assert_eq!(f.shape_cache.lock().unwrap().plans.entries.len(), 1);
}

#[test]
#[cfg(feature = "perf")]
fn distinct_strings_reuse_one_plan_despite_glyph_cache_churn() {
    let f = face();
    crate::perf::reset();
    for _ in 0..2 {
        for i in 0..512 {
            let _ = f.shape(&format!("Item {i}"));
        }
    }
    assert_eq!(crate::perf::count("font.shape"), 1024);
    assert_eq!(crate::perf::count("font.shape_plan"), 1);
}

#[test]
#[cfg(feature = "perf")]
fn glyph_cache_clock_rollover_keeps_immutable_plans() {
    let f = face();
    crate::perf::reset();
    let _ = f.shape("Item");
    f.shape_cache.lock().unwrap().clock = u64::MAX;
    assert_eq!(f.shape("Another"), control(&f, "Another", None));
    assert_eq!(crate::perf::count("font.shape_plan"), 1);
    assert_eq!(f.shape_cache.lock().unwrap().plans.entries.len(), 1);
}
