//! turbo-dom#74: `HTMLTrackElement` and its `TextTrack` against the Chrome 156 control
//! `html_track_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs): the interfaces and their members, the
//! reflected `src` / `kind` / `srclang` / `label` / `default`, `readyState` NONE without a media
//! element, the `track` object's kind / label / language / id / mode and its empty cue lists.

/// Rows that depend on turbo-dom#73 (P5b-X7, branch `consol/p5b-x7`): the shared reflected
/// `src` getter does not resolve against `<base href>` yet, so `track.src` returns the raw
/// attribute. They must still differ here; once #73 lands they match Chrome and this list is
/// emptied.
const TURBO_DOM_73_ROWS: &[&str] = &[".emptySrc.0:", ".parsed.4:", ".plain.4:", ".setters.5:"];

#[test]
fn html_track_matches_chrome156_control() {
    let found = super::shadow_events_tests::control_mismatches(
        include_str!("html_track_probe.js"),
        include_str!("html_track_cases.json"),
        include_str!("html_track_chrome156.json"),
    );
    let unexpected: Vec<_> = found
        .iter()
        .filter(|row| !TURBO_DOM_73_ROWS.iter().any(|gap| row.starts_with(gap)))
        .collect();
    let resolved: Vec<_> = TURBO_DOM_73_ROWS
        .iter()
        .filter(|gap| !found.iter().any(|row| row.starts_with(*gap)))
        .collect();
    assert!(
        unexpected.is_empty() && resolved.is_empty(),
        "unexpected mismatches: {unexpected:#?}\nturbo-dom#73 rows now matching (empty TURBO_DOM_73_ROWS): {resolved:?}"
    );
}
