//! Install cost: a normal install (fresh isolate + `install_with_html`) against
//! `install_from_snapshot` (turbo-dom#249). Run with
//! `cargo run --release -p turbo-dom-browser-env --example snapshot_install_cost`.
//! Prints min / median over 30 installs of each, plus the one-time blob build.
use std::time::{Duration, Instant};
use turbo_dom_browser_env::browser_env;

const PAGE: &str =
    "<!doctype html><title>t</title><body><div id=a class=x>hi</div><form><input required></form>";
const RUNS: usize = 30;

fn stats(mut samples: Vec<Duration>) -> (Duration, Duration) {
    samples.sort();
    (samples[0], samples[samples.len() / 2])
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn main() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let started = Instant::now();
    browser_env::snapshot_blob().expect("the process blob builds");
    let build = started.elapsed();

    let mut normal = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        let isolate = &mut v8::Isolate::new(Default::default());
        {
            v8::scope!(let scope, isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            browser_env::install_with_html(scope, PAGE);
        }
        normal.push(started.elapsed());
        browser_env::reset();
    }

    let mut snapshot = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        let window = browser_env::install_from_snapshot(PAGE).expect("a blob");
        snapshot.push(started.elapsed());
        drop(window);
        browser_env::reset();
    }

    let (normal_min, normal_median) = stats(normal);
    let (snapshot_min, snapshot_median) = stats(snapshot);
    println!("blob build (once per process): {:.1} ms", ms(build));
    println!(
        "install_with_html (isolate + install): min {:.2} ms, median {:.2} ms",
        ms(normal_min),
        ms(normal_median)
    );
    println!(
        "install_from_snapshot:                 min {:.2} ms, median {:.2} ms",
        ms(snapshot_min),
        ms(snapshot_median)
    );
    println!(
        "median speedup: {:.1}x",
        normal_median.as_secs_f64() / snapshot_median.as_secs_f64()
    );
}
