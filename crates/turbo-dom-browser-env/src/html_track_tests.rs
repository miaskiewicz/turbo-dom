//! turbo-dom#74: `HTMLTrackElement` and its `TextTrack` against the Chrome 156 control
//! `html_track_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs): the interfaces and their members, the
//! reflected `src` / `kind` / `srclang` / `label` / `default` (resolved against `<base href>`,
//! turbo-dom#73), `readyState` NONE without a media element, the `track` object's kind / label /
//! language / id / mode and its empty cue lists.

#[test]
fn html_track_matches_chrome156_control() {
    let found = super::shadow_events_tests::control_mismatches(
        include_str!("html_track_probe.js"),
        include_str!("html_track_cases.json"),
        include_str!("html_track_chrome156.json"),
    );
    assert!(found.is_empty(), "mismatches vs Chrome 156: {found:#?}");
}
