//! Public `matches_scoped` dense-sibling benchmark; parsing is warmed before timing.
use std::hint::black_box;
use std::time::Instant;
use turbo_dom::Tree;

fn main() {
    for count in [256, 512, 1024, 2048, 4096] {
        let tree = Tree::parse(&format!("<main>{}</main>", "<span></span>".repeat(count)));
        let scope = tree.query_selector("main").unwrap();
        let nodes = tree.query_selector_all("main > span");
        let selector = "span:nth-of-type(3n+1)";
        black_box(tree.matches_scoped(nodes[0], selector, scope));
        let mut samples = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            let matched = nodes
                .iter()
                .filter(|&&node| black_box(tree.matches_scoped(node, selector, scope)))
                .count();
            assert_eq!(matched, count.div_ceil(3));
            samples.push(start.elapsed().as_nanos());
        }
        samples.sort_unstable();
        println!(
            "{{\"siblings\":{count},\"matches\":{},\"median_ns\":{},\"minimum_ns\":{}}}",
            count.div_ceil(3),
            samples[3],
            samples[0]
        );
    }
}
