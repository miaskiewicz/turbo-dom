//! turbo-dom#339: HTMLMediaElement.captureStream() and MediaStream, resource selection restarted
//! by `<source>` insertion, TextTrackList's addtrack / removetrack / change events (TrackEvent),
//! automatic text track selection and the processing model of a track without a src, against
//! the Chrome 156 control `html_media_events_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs).
use super::interface_parity_tests::{control, mismatches, run_probe_with};

#[test]
fn html_media_events_match_chrome156_control() {
    let probe = include_str!("html_media_events_probe.js");
    let cases = include_bytes!("html_media_events_cases.json");
    let expected = control(
        probe,
        cases,
        include_str!("html_media_events_chrome156.json"),
    );
    let found = mismatches("", &run_probe_with(probe, cases, ""), &expected);
    assert!(
        found.is_empty(),
        "mismatches vs Chrome 156:\n{}",
        found.join("\n")
    );
}

/// Where Chrome would fetch, the binding's network never answers: a track with a src stays
/// LOADING once its mode starts the track processing model, a src removed fails it, a new src
/// reloads it, and a src change while a fetch runs cancels that fetch (ERROR and error, the new
/// fetch still running). Chrome 156 does the same against a server that never answers.
#[test]
fn a_track_with_a_src_stays_loading() {
    let probe = r#"(async () => {
      const tick = () => new Promise((resolve) => setTimeout(resolve, 30));
      const v = document.createElement('video'), t = document.createElement('track'), log = [];
      for (const type of ['load', 'error']) t.addEventListener(type, () => log.push(type + t.readyState));
      t.src = 'a.vtt';
      v.append(t);
      t.track.mode = 'hidden';
      await tick();
      const loading = [t.readyState, log.slice()];
      t.removeAttribute('src');
      await tick();
      loading.push(t.readyState, log.slice());
      t.src = 'b.vtt';
      loading.push(t.readyState);
      await tick();
      loading.push(t.readyState, log.slice());
      t.src = 'c.vtt';
      await tick();
      loading.push(t.readyState, log.slice());
      return loading;
    })()"#;
    let cases = br#"[{"html":"<!doctype html><body>","documentBase":"http://localhost/"}]"#;
    assert_eq!(
        run_probe_with(probe, cases, ""),
        serde_json::json!([
            1,
            [],
            3,
            ["error3"],
            3,
            1,
            ["error3"],
            3,
            ["error3", "error3"]
        ])
    );
}
