These controls were captured from a fresh installed Chrome profile. The JSON records
its browser/V8 version, exact probe and collector hashes, and errors. The native
integration test rejects a stale probe hash and compares state matching against the
recorded result. No network site or anti-bot service is used.

From this repository, with the sibling turbo-surf checkout available:

```sh
node ../turbo-surf/harness/detection/authored-upload-chrome.mjs \
  crates/turbo-dom/tests/fixtures/custom-state/probe.js \
  crates/turbo-dom/tests/fixtures/custom-state/chrome155.json
cargo test -p turbo-dom --test custom_states
```

The collector starts its own localhost HTTPS fixture and removes its temporary
Chrome profile. The control checks exact DOMString keys, CSS preprocessing and
identifier parsing, constructor-owned clone/import state, state preservation on
adoption, and absence of attribute/mutation effects. Tree migration itself belongs
to the embedding DOM binding: enumerate/reapply states only for successful adoption;
clone/import create fresh state. Enumeration order is intentionally unspecified;
the embedding CustomStateSet retains its own observable insertion order.
