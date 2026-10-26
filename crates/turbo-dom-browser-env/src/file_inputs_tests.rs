use super::*;
use sha2::{Digest, Sha256};

// The standalone binding has no File or FormData backend. This models an embedder's
// branded File (private name/type/size/lastModified, lowercased printable type) and a
// FormData over the binding's own entry list; turbo-surf runs the same probe against its
// production File and FormData.
const HOST: &str = r#"(() => {
  const blobs = new WeakMap(), files = new WeakMap(), forms = new WeakMap();
  const read = (map, value) => { const state = map.get(value); if (!state) throw new TypeError("Illegal invocation"); return state; };
  const bytes = (part) => typeof part === "string" ? part.length : part instanceof Uint8Array ? part.length : blobs.has(part) ? blobs.get(part).size : String(part).length;
  class Blob {
    constructor(parts = [], options = {}) {
      let size = 0; for (const part of parts) size += bytes(part);
      const type = String(options.type || "");
      blobs.set(this, { size, type: /[^\x20-\x7e]/.test(type) ? "" : type.toLowerCase() });
    }
    get size() { return read(blobs, this).size; }
    get type() { return read(blobs, this).type; }
  }
  class File extends Blob {
    constructor(parts, name, options = {}) {
      super(parts, options);
      files.set(this, { name: String(name), lastModified: options.lastModified === undefined ? Date.now() : Number(options.lastModified) });
    }
    get name() { return read(files, this).name; }
    get lastModified() { return read(files, this).lastModified; }
    get webkitRelativePath() { read(files, this); return ""; }
  }
  Object.defineProperty(Blob.prototype, Symbol.toStringTag, { value: "Blob", configurable: true });
  Object.defineProperty(File.prototype, Symbol.toStringTag, { value: "File", configurable: true });
  class FormData {
    constructor(form, submitter) {
      forms.set(this, []);
      if (form === undefined) return;
      const rows = __fdBegin(form, submitter);
      try { for (const [name, value] of rows) forms.get(this).push([name, value]); __fdEvent(form, this); } finally { __fdFinish(form); }
    }
    get(name) { const row = read(forms, this).find((entry) => entry[0] === name); return row ? row[1] : null; }
    *[Symbol.iterator]() { yield* read(forms, this).map((row) => [row[0], row[1]]); }
  }
  globalThis.Blob = Blob; globalThis.File = File; globalThis.FormData = FormData;
  return (value) => files.has(value) ? Object.freeze({ kind: "file", value, name: files.get(value).name })
    : forms.has(value) ? Object.freeze({ kind: "formdata", entries: Object.freeze(forms.get(value).map((row) => Object.freeze(row.slice()))) }) : null;
})()"#;

fn install_host(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__fdBegin", |scope, args, mut rv| {
        if let Some(entries) = begin_form_data_constructor_entries(scope, args.get(0), args.get(1))
        {
            rv.set(entries.into());
        }
    });
    bind_method(scope, global, "__fdFinish", |scope, args, _rv| {
        finish_form_data_constructor(scope, args.get(0))
    });
    bind_method(scope, global, "__fdEvent", |scope, args, _rv| {
        if let Ok(form) = v8::Local::<v8::Object>::try_from(args.get(0)) {
            dispatch_formdata_event(scope, form, args.get(1));
        }
    });
    let source = v8::String::new(scope, HOST).unwrap();
    let reader = v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap();
    assert!(install_form_value_reader(
        scope,
        v8::Local::<v8::Function>::try_from(reader).unwrap()
    ));
    capture_file_host(scope);
}

/// Evaluate a probe (sync or a promise) after installing the modeled host, draining
/// a FIFO setTimeout queue shared with the binding's private scheduler.
fn run_probe(probe: &str, html: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    run_js(
        scope,
        "globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};",
    );
    capture_popover_scheduler(scope);
    install_host(scope);
    let source = format!(
        "Promise.resolve({}).then(JSON.stringify)",
        probe.trim().trim_end_matches(';')
    );
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap_or_else(|| {
            panic!(
                "probe threw: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            )
        });
    let step = v8::String::new(scope, "__tasks.length&&__tasks.shift()()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..10_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending {
            break;
        }
        step.run(scope);
    }
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Leaf paths where the binding and the saved Chrome control disagree.
fn differences(
    path: &str,
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    out: &mut Vec<String>,
) {
    use serde_json::Value::{Array, Object};
    match (actual, expected) {
        (Array(a), Array(b)) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                differences(&format!("{path}[{index}]"), a, b, out);
            }
        }
        (Object(a), Object(b)) if a.len() == b.len() => {
            for (key, b) in b {
                match a.get(key) {
                    Some(a) => differences(&format!("{path}.{key}"), a, b, out),
                    None => out.push(format!("{path}.{key}: missing")),
                }
            }
        }
        _ if actual != expected => out.push(format!("{path} = {actual} (Chrome {expected})")),
        _ => {}
    }
}

/// The saved control, after checking it was captured from this exact probe and case list.
fn control(fixture: &str, probe: &str, cases: &[u8]) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["v8"], "15.6.75.19");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases))
    );
    control
}

/// Compare with Chrome, allowing exactly the pinned `path = binding value` deltas.
fn assert_matches(actual: &serde_json::Value, expected: &serde_json::Value, known: &[&str]) {
    let mut out = Vec::new();
    differences("", actual, expected, &mut out);
    let mut unexpected = Vec::new();
    for difference in &out {
        if !known.iter().any(|gap| difference.starts_with(gap)) {
            unexpected.push(difference.clone());
        }
    }
    let missing = known
        .iter()
        .filter(|gap| !out.iter().any(|difference| difference.starts_with(*gap)))
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty() && missing.is_empty(),
        "unexpected:\n{}\nresolved known gaps: {missing:?}",
        unexpected.join("\n")
    );
}

#[test]
fn file_inputs_match_chrome156_control() {
    let probe = include_str!("file_inputs_probe.js");
    let cases = include_bytes!("file_inputs_cases.json");
    let control = control(include_str!("file_inputs_chrome156.json"), probe, cases);
    let html = serde_json::from_slice::<serde_json::Value>(cases).unwrap()[0]["html"]
        .as_str()
        .unwrap()
        .to_owned();
    let actual = run_probe(probe, &html);
    assert_matches(&actual, &control["result"][0]["result"], FILE_INPUT_GAPS);
}

#[test]
fn text_state_value_sanitization_matches_chrome156_control() {
    let probe = include_str!("input_sanitization_probe.js");
    let cases = include_bytes!("input_sanitization_cases.json");
    let control = control(
        include_str!("input_sanitization_chrome156.json"),
        probe,
        cases,
    );
    let actual = run_probe(
        probe,
        "<!doctype html><html><head></head><body></body></html>",
    );
    assert_matches(&actual, &control["result"][0]["result"], &[]);
}

/// Measured Chrome behavior the binding does not reproduce, as `path = binding value`.
///
/// Blink's KURL percent-escapes a space inside a special URL's host ("http://a b/" is
/// "http://a%20b/"); the WHATWG parser rejects it, so getData("url") takes the next line.
///
/// Blink tells strict from sloppy writes (and Reflect.set/defineProperty from ordinary
/// definitions) through PropertyCallbackInfo::ShouldThrowOnError, which Rusty V8 always
/// reports as false, as for document.all. The binding keeps sloppy semantics and the
/// throwing Object.defineProperty contract; strict writes still throw a TypeError, in
/// V8's words, and the two Reflect results differ.
pub(super) const FILE_INPUT_GAPS: &[&str] = &[
    ".strings[7][0] = \"http://c/\"",
    ".strictMutation[0][1] = \"Cannot assign to property '0' of #<FileList>\"",
    ".strictMutation[1][1] = \"Cannot assign to property '3' of #<FileList>\"",
    ".strictMutation[2][1] = \"Cannot delete property '0' of #<FileList>\"",
    ".strictMutation[4] = false",
    ".strictMutation[5] = false",
    ".strictMutation[6] = [\"TypeError\",\"Failed to set an indexed property [0] on 'FileList': Index property setter is not supported.\"]",
    ".itemListMutation[1][1] = \"Cannot assign to property '0' of #<DataTransferItemList>\"",
];
