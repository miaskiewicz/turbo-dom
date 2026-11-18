use super::*;

fn control_direction(text: &str) -> bool {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    buffer.direction() == rustybuzz::Direction::RightToLeft
}

fn control_shape(face: &FontFace, text: &str, rtl: Option<bool>) -> Vec<ShapedGlyph> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    if let Some(rtl) = rtl {
        buffer.set_direction(if rtl {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
    }
    collect_glyphs(&rustybuzz::shape(
        &face.faces.borrow_dependent().rb,
        &[],
        buffer,
    ))
}

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
fn ascii_direction_matches_rustybuzz_for_controls_pairs_and_long_sequences() {
    assert_eq!(guessed_rtl(""), control_direction(""));
    for first in 0..128u8 {
        let text = char::from(first).to_string();
        assert_eq!(guessed_rtl(&text), control_direction(&text), "{text:?}");
        for second in 0..128u8 {
            let text = String::from_utf8(vec![first, second]).unwrap();
            assert_eq!(guessed_rtl(&text), control_direction(&text), "{text:?}");
        }
    }
    for seed in 0..256u32 {
        let text: String = (0..96u32)
            .map(|index| char::from(((seed * 31 + index * 73) % 128) as u8))
            .collect();
        assert_eq!(guessed_rtl(&text), control_direction(&text), "{seed}");
    }
}

#[test]
fn non_ascii_keeps_rustybuzz_script_guessing_including_direction_marks() {
    for text in [
        "אבג",
        "مرحبا",
        "a אבג",
        "אבג 123",
        "é",
        "漢字",
        "👩\u{200d}💻",
        "\u{200e}אבג",
        "\u{200f}abc",
        "\u{202e}abc",
        "\u{2067}123\u{2069}",
        "e\u{301}",
        "\u{fffd}",
    ] {
        assert_eq!(guessed_rtl(text), control_direction(text), "{text:?}");
    }
}

#[test]
fn ascii_shortcut_preserves_real_glyphs_forced_domains_and_original_units() {
    let face = face();
    let ascii: String = (0..128u8).map(char::from).collect();
    for text in [
        ascii.as_str(),
        "",
        "Invoice 123.45",
        "abc",
        "AV fi ffi",
        "\t\n\r",
        " !? 123",
        "אבג",
        "مرحبا",
        "\u{200e}abc",
        "\u{200f}abc",
        "a\u{fffd}b",
    ] {
        for rtl in [true, false, true] {
            assert_eq!(
                face.shape_in(text, rtl),
                control_shape(&face, text, Some(rtl)),
                "{text:?} rtl={rtl}"
            );
            assert_eq!(
                face.shape(text),
                control_shape(&face, text, None),
                "raw {text:?}"
            );
        }
    }
    let original = [0x61, 0xd800, 0x62];
    assert_eq!(
        face.shape_original_utf16(&original),
        control_shape(&face, "a\u{fffd}b", None)
    );
    assert!(face
        .shape_cache
        .lock()
        .unwrap()
        .units_entries
        .contains_key(original.as_slice()));
}

#[cfg(feature = "perf")]
#[test]
fn cached_ascii_shape_in_does_not_allocate_a_direction_guess_buffer() {
    let face = face();
    let text = "Invoice 123.45";
    let ltr = face.shape_in(text, false);
    let rtl = face.shape_in(text, true);
    assert_ne!(ltr, rtl);
    crate::perf::reset();
    for _ in 0..128 {
        assert_eq!(face.shape_in(text, false), ltr);
        assert_eq!(face.shape_in(text, true), rtl);
    }
    assert_eq!(crate::perf::count("font.direction_guess_buffer"), 0);
    assert_eq!(crate::perf::count("font.shape"), 0);
    assert_eq!(
        face.shape_in("אבג", false),
        control_shape(&face, "אבג", Some(false))
    );
    assert_eq!(crate::perf::count("font.direction_guess_buffer"), 1);
}
