#[path = "../../crates/turbo-dom-raster/src/avif.rs"]
mod avif;
use std::sync::{Arc, Barrier};
fn main() {
    let valid =
        std::fs::read("crates/turbo-dom-layout/tests/fixtures/images/formats/q.avif").unwrap();
    let payload = turbo_dom_layout::image::avif_payload(&valid).unwrap();
    let at = valid
        .windows(payload.color.len())
        .position(|p| p == payload.color)
        .unwrap();
    let mut corrupt = valid.clone();
    corrupt[at + payload.color.len() / 2..at + payload.color.len()].fill(0xff);
    std::fs::write(".perf-evidence/avif-concurrency/corrupt.avif", &corrupt).unwrap();
    let inputs = Arc::new((valid, corrupt));
    let barrier = Arc::new(Barrier::new(17));
    println!(
        "CONCURRENCY-START callers=16 calls_per_caller=128 decoder_threads=2 max_frame_delay=2"
    );
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let inputs = Arc::clone(&inputs);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                for index in 0..128 {
                    if index % 2 == 0 {
                        let image =
                            avif::decode(&inputs.0).expect("unchanged valid fixture decodes");
                        assert_eq!((image.width, image.height), (64, 40));
                    } else {
                        assert!(
                            avif::decode(&inputs.1).is_none(),
                            "known corrupted stream rejected"
                        );
                    }
                }
            })
        })
        .collect();
    barrier.wait();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("CONCURRENCY-DONE valid=1024 broken=1024 total=2048");
}
