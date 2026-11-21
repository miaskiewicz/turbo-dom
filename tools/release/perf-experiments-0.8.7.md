# turbo-dom v0.8.7 perf experiments

Measurement method: `parse_html_soa` / `parse_html_document` / `parse_html_document_count`
over `bench/fixtures.mjs`, minimum of 5-6 ABBA rounds, load check before and after,
counterbalanced (A→B→B→A). The harness source and ABBA runner are in
`/private/tmp/claude-501/.../scratchpad/perf087/` for this session only.

## Noise floor at load ≈ 5 (quiet), 5 ABBA rounds

| fixture | soa min Δ A vs A' |
|---|---:|
| deep-nested | +0.6% |
| malformed | −2.6% |
| real-storybook | −0.7% |
| small | +0.0% |
| ssr-large | −0.4% |

Big fixtures ±1%; the two micro fixtures swing ±3% because their per-op
time (3–4 µs) is comparable to one iteration of the timer loop.

## Experiment log

### Exp 1 — arena capacity hint + attr-flat-table reserve + merge_text inline — **DROP**

Combined three allocation-reduction tweaks:

- `Arena::with_capacity(bytes/32)` sizes the node Vec once from html length.
- `soa_builder(nodes)` also reserves `attr_name_id` / `attr_value_id` / `attr_prefix_id`
  at `nodes/2` capacity.
- `arena_merge_text` gets an explicit `at == NIL` early return + `#[inline]`.

Numbers, 6 ABBA rounds:

| fixture | soa Δ | soa+cm Δ | count Δ (RcDom path, control) |
|---|---:|---:|---:|
| deep-nested | −0.9% | −0.8% | −1.2% |
| real-storybook | **−2.3%** | −1.0% | −0.4% |
| ssr-large | −0.3% | −1.3% | +0.7% |
| small | −3.2% | −3.2% | +4.8% |
| malformed | +0.0% | −2.8% | +0.0% |

Only `real-storybook` clears the 2% bar on the primary measure (`soa`).
`ssr-large` barely moves (−0.3%). `small` and `malformed` swing in opposite
directions by ~3%, consistent with the noise floor. The RcDom control (`count`)
shifts without the arena path being touched, so the real signal is inside the
noise band of this run. Dropped.

### Exp 2 — `Arena::with_capacity(html.len() / 32)` ALONE — **DROP**

Isolated the single tweak from Exp 1 that was easiest to justify. 8 ABBA rounds,
load 2.89 → 3.92 across the run (background drift).

| fixture | count Δ | soa Δ | soa+cm Δ | doc Δ |
|---|---:|---:|---:|---:|
| deep-nested | +5.8% | +4.5% | +2.7% | +5.9% |
| malformed | +0.0% | +2.8% | +2.9% | +4.5% |
| real-storybook | +5.5% | +5.3% | +5.4% | +6.2% |
| small | +4.8% | +3.3% | +3.3% | +3.1% |
| ssr-large | +5.7% | +5.2% | +4.3% | +3.5% |

Every fixture slower. A ±5 % load drift during the run is most of it — malformed
and small fixtures have html lengths of a few hundred bytes, so `html.len()/32`
reserves 10-20 slots, which can't plausibly regress their parse by 3-5 %. The
honest conclusion is the signal is below the current load's measurement floor,
not that this experiment is a net loss. Still dropped from v0.8.7: a tweak the
harness can't prove non-regressing at the 2 % bar does not ship.

### Not run

Session ran out of honest measurement time: load swung from 3 → 7 mid-sweep on
several runs, and the parser is already well-optimized after the 0.8.6 arena
sink work, so the easy wins are gone. Candidates still worth trying when the
machine is cold:

- Replace `tag_map`/`attr_prefix_map` FxHashMaps with a hash-keyed index
  (store `u64 hash → u32 index`, re-compare by slice for collisions) to save
  the owned-String key per intern miss.
- `#[inline(always)]` on `arena_push`, `arena_link_last`, `arena_link_before`.
- Flatten `alloc_arena` into a tail-recursive walk that uses a stack-allocated
  previous-sibling pointer per level instead of a `Vec<i32> last_child`.
- `set_element`: batch the three `attr_*_id.push` calls into a single-struct
  temporary written once with `std::ptr::write`.
- SoA `strings`: move text node content via `StrTendril::try_subtendril` +
  `to_string` only on read-side (changes public API — likely not acceptable).
- JS-side `bench:query` scan for hotspots the recent commits may have shifted.

Of these, the first two look cheapest; the fourth and fifth would need a
deliberate budget. None are expected to break the 2% bar reliably on their own —
tight parser code at this stage tends to ship combined only.
