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

### Exp 3 — remove `SoaBuilder` upfront `reserve(nodes)` — **DROP (also: current reserves earn their keep)**

Opposite angle from Exp 1/2: maybe the per-field `reserve(nodes)` block on 12
Vecs is wasted work on big inputs (`nodes` is only an upper bound). Took them
all out. 8 ABBA rounds, load spike 5–15 during run (noisier than Exp 2).

| fixture | count Δ | soa Δ | soa+cm Δ | doc Δ |
|---|---:|---:|---:|---:|
| deep-nested | −1.2% | +0.2% | +0.3% | +0.2% |
| malformed | +0.0% | **+18.9%** | **+18.9%** | +0.0% |
| real-storybook | +2.0% | +3.0% | +1.8% | +1.7% |
| small | +0.0% | **+16.1%** | **+12.9%** | +0.0% |
| ssr-large | −1.1% | +3.0% | +2.6% | +0.4% |

Clear negative signal on the SoA path for tiny inputs (12–19% slower) and ~3 %
on real-storybook / ssr-large. The small / malformed regression is mechanical:
without the pre-reserve the 12 Vecs each start at capacity 0 and reallocate
several times, which dominates a 3 µs parse. `count` / `doc` paths don't change
because they use the `RcDom` tree (no `SoaBuilder`). Reverted. The current
reserves are correctly sized; don't remove them.

### Exps 4–10 — various micro-opts, all dropped

Load during this session ranged 3 – 38. Measurements only usable at the lower
end, and even then the ±2 % bar was within noise. Each experiment ran 4–6 ABBA
rounds, and most promising-looking first runs did not replicate on a second
pass with different load timing.

| # | change | verdict |
|---|---|---|
| 4 | pre-populate `tag_map`/`attr_name_map` with common HTML idents at `soa_builder()` | DROP — kills small/malformed (+35 %, +45 % on soa) from 45 upfront `to_string` allocations |
| 5 | cache `empty_prefix_id` as `Option<u32>` field, skip hashmap lookup in `set_element` | DROP — ±1 % both ways, within noise |
| 6 | `Vec::with_capacity(nodes.len())` for `pending` in `alloc_arena` | DROP — one run −16 % on ssr-large, another run +20 %; pure ABBA-vs-load bias |
| 7 | `#[inline]` on `arena_push`, `arena_link_last`, `arena_merge_text` | DROP — looked ±3 % win on one run, invalidated by a second ABBA vs base (flat ±1 %) |
| 8 | #7 + `#[inline]` on all four `intern_*` fns | DROP — vs base flat, i.e. the #7 win was noise |
| 9 | bigger fx-hashmap initial capacities (tag/attr-name → 256, values → 512) | DROP — first run looked −9 % across the board (load bias), replication flat ±2 % |
| 10 | `Vec::with_capacity(64)` for `pending` alone | DROP — ssr-large −1.9 % / −2.4 %, right at the 2 % bar, not reliable |

### html5ever patch inspection

`html5ever::tree_builder::TreeBuilder::insert_element` (and five sibling call sites
at lines 839, 872, 1020, 1454, 1545) does `attrs.clone()` before calling
`create_element_with_flags`, then still iterates `attrs` for the form-associatable
check and drops it at end of scope. A `[patch.crates-io]` fork could reorder the
form check to run first on `&attrs` and then move the Vec into `create_element`
with no clone. From the baseline profile this would save ≈1 % total (24+58
samples / 8363 = 1 %), i.e. the clone + its subsequent free are a 1 % band and a
patch would not clear the 2 % bar on its own either. Shelved — the patch is a
multi-site refactor of an upstream crate for sub-2 % return, and would add a
lockfile divergence that CI, PUBLISHING and downstream crates (turbo-dom, -layout,
-raster, -browser-env) would all have to track.

### Honest conclusion

The parser is tight after 0.8.6's arena work. Under the measurement conditions
this session had, nothing cleared the 2 % reliable-win bar. Ship v0.8.7 on the
#539 fix alone; perf work belongs in a dedicated sweep on a cold machine (no
other sessions mid-benchmark, idle load < 2).

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
