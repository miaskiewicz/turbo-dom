//! `TURBO_RUST_DOM_LOG` debugging output: with the variable set, reads of members the
//! binding does not implement are counted per name and `dump_missing` (and `reset`) print
//! them, most-read first (ties by name), then clear the log; an `in` check counts as a read
//! and symbols / implemented members do not. The variable is read once per process, so the
//! test re-runs itself as a child process with it set and asserts the child's stderr.
use turbo_dom_browser_env::browser_env;

const CHILD: &str = "TURBO_DOM_MISSING_LOG_CHILD";

fn child() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    assert!(browser_env::log_enabled());
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_with_html(scope, "<!doctype html><p id=p></p>");
    // Drop whatever the bootstrap itself read; a dump of the empty log prints nothing.
    browser_env::dump_missing();
    eprintln!("--- empty");
    browser_env::dump_missing();
    eprintln!("--- reads");
    let code = v8::String::new(
        scope,
        "const p = document.getElementById('p'); [p.zetaMissing, p.alphaMissing, p.zetaMissing, p[Symbol.iterator], 'betaMissing' in p, p.id]",
    )
    .unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    browser_env::dump_missing();
    eprintln!("--- after dump");
    let code = v8::String::new(scope, "document.getElementById('p').gammaMissing").unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    browser_env::reset();
    eprintln!("--- done");
    // A bootstrap script that throws (here: the binding namespace was frozen before
    // install) is logged, and the install still completes.
    let frozen = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, frozen);
    let code = v8::String::new(
        scope,
        "Object.defineProperty(globalThis, Symbol.for('ts'), { value: Object.freeze({}) })",
    )
    .unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    browser_env::install_with_html(scope, "<!doctype html>");
    browser_env::reset();
    eprintln!("--- bootstrap done");
}

#[test]
fn missing_member_log_counts_and_prints_unimplemented_reads() {
    if std::env::var_os(CHILD).is_some() {
        child();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "missing_member_log_counts_and_prints_unimplemented_reads",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .env("TURBO_RUST_DOM_LOG", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    let empty = stderr.find("--- empty\n").expect("child ran") + "--- empty\n".len();
    assert!(stderr[empty..].starts_with("--- reads\n"), "{stderr}");
    let start = stderr.find("--- reads\n").expect("child ran") + "--- reads\n".len();
    let end = stderr.find("--- done\n").expect("child finished");
    let tail = &stderr[end..stderr.find("--- bootstrap done\n").expect("bootstrap ran")];
    assert!(
        tail.lines()
            .any(|line| line.starts_with("[rust-dom] bootstrap threw: TypeError")),
        "{tail}"
    );
    assert_eq!(
        &stderr[start..end],
        "[rust-dom] 3 missing DOM members (name×accesses):\n\
         [rust-dom]   zetaMissing ×2\n\
         [rust-dom]   alphaMissing ×1\n\
         [rust-dom]   betaMissing ×1\n\
         --- after dump\n\
         [rust-dom] 1 missing DOM members (name×accesses):\n\
         [rust-dom]   gammaMissing ×1\n"
    );
}
