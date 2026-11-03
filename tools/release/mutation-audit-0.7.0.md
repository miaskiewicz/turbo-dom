# Mutation audit, 0.7.0 round-2 fixes

cargo-mutants 27.1.0 on PR #232 (base `55dd9d38`), branch `wip/r3-mutants`.

## Method

- **Scope.** Only the lines the round-2 fix commits wrote. `git blame` of the current tree
  kept the lines last touched by the fix commits (`ad877eaf` radius, `1327e2c2` clone
  bookkeeping, `b6d4000a` ordinals, `b50028f4` UAX #50, `922b0bfe` range step,
  `06ebf250` positive ints, `9dd30b4d` inset/outset, `fe6dc7f2` gradients, `c2d1a6df` +
  `7ad7c982` containment, `f3f1079c` image input, `8f003c7d` + `a215033c` generic
  families). Those lines became a synthetic diff for `cargo mutants --in-diff`.
- **Run.** `cargo mutants -p turbo-dom-layout --test-package turbo-dom-layout
  --test-package turbo-dom-raster --in-diff <blame.diff> -j 5 --timeout 600`. That is 382
  mutants. `BorderRadii::fit` (fragment.rs) predates round 2, so it got its own run with
  `--file fragment.rs --re BorderRadii::fit` (15 mutants).
- **Follow-up.** Every survivor got a test that kills it, or a written reason why it is
  equivalent. Kills were checked by mutating the code by hand and running the targeted
  test, or by re-running cargo-mutants on the touched functions.
- **Totals.** Run 1: 328 caught, 23 missed, 7 timeouts (all infinite unrolling, so they
  count as caught), 24 unviable. `fit`: 15 of 15 caught.

## Per item

| # | Item | Functions | Mutants (viable) | Caught | Missed | Test added |
|---|------|-----------|------------------|--------|--------|------------|
| 1 | border-radius infinite/huge clamp | `value.rs` `split_unit`, `CornerRadius::resolve`; `fragment.rs` `BorderRadii::fit` | 22 + 15 | 22 (1 timeout) + 15 | 0 | none needed |
| 2 | box-decoration-break clone running reserve | `inline.rs` `Placer::{sync_clones, clone_reserve, cloned_open, wrap_before, flush, place, attach_trailing_close}`, `apply_clone_edges` | 19 | 11 | 8 | **real bug, fixed**: paint fixtures `inline-clone-reserve`, `inline-clone-reserve-text`, `inline-slice-close-glued`, `inline-close-glued(-nested)` |
| 3 | i32 ordinal clamp | `list_marker.rs` `ListCounters::ordinal` | 3 | 3 | 0 | none needed |
| 4 | UAX #50 vertical orientation | `inline_boxes.rs` `sets_upright`, `in_ranges`, `inherits_orientation`; `inline.rs` `UprightText::sets_after` | 16 | 16 | 0 | none needed (the table itself is data, no operators) |
| 5 | range step snapping | `form_paint.rs` `step_align`, `step_of`, `range`, `native` | 50 | 47 | 3 | paint fixture `form-native-range-step-base` + unit assertion (kills 1); 2 equivalent |
| 6 | `positive()` i32 parsing | `form_controls.rs` `positive_int`, `intrinsic_size` | 13 | 13 | 0 | none needed |
| 7 | #257 inset/outset shading | `value.rs` `border_side_style`, `border_style_colour`, `shade_side`, `lighten`, `scale_colour`, `resolve_borders` | 27 | 27 | 0 | none needed |
| 8 | GradientSpec stop resolution | `value.rs` `GradientSpec::resolve`, `repeated`, `average`, `cut_to_line`, `parse_gradient_stop`, `linear_gradient_*`, `channels`; `background.rs` `layer`; `block.rs` `prepend_background_layers`; `inline_boxes.rs` `background_layers` | 115 | 107 (6 timeouts) | 8 | unit test `a_too_fine_repeating_gradient_paints_its_length_weighted_average` (kills 5); 3 equivalent |
| 9 | container-type size containment | `boxgen.rs` `Containment::of`, `LayoutBox::{containment, ignores_size}`; `block.rs` `atom_baseline`, `contained_flow_baseline`, `establishes_bfc`; `flex.rs` `natural_width`, `min_content_width` | 38 | 36 | 2 | **real bug, fixed**: live fixture `contain-flow-baselines` (HTML and live paths) |
| 10 | input[type=image] | `boxgen.rs` `img_src`, `is_image_input`, `is_img`, `image_of`, `image_input_alt`, `img_alt_text`, `broken_image_size`; `html_layout.rs` `record_img_data_uri` | 33 | 31 | 2 | live fixture `src-not-image` + HTML-path assertion in `layout_boxes_loads_an_image_inputs_data_uri` (kills 1); 1 equivalent |
| 11 | sans-serif/monospace generic metrics | `registry.rs` `html_generic_candidates`, `FontRegistry::alias_generics*` | 11 | 11 | 0 | none needed |

## Survivors

### Item 2: real bug, fixed test-first

`Placer::clone_reserve -> 0.0` survived the whole suite. No test had a line where the
reserve changed the break. Chrome 156 (`inline-clone-reserve`) does not reserve room for a
cloned box's repeated end edge: two atoms that fit with the start edge stay on the line,
and the end edge overflows it.

Taking the reserve out uncovered a second, older bug (`inline-slice-close-glued`, sliced
and cloned boxes alike). Chrome has no break opportunity between an atom and the end tag
right after it, so the atom wraps when the atom and the box's end edge do not both fit.
The Placer placed that end tag as its own word. The fix:

- removes the per-word clone reserve;
- gives each `Word` a `close_after` (the end edges of the closing-only words glued to it);
- uses `close_after` in `needs_wrap`.

The perf test `clone_bookkeeping_grows_linearly_with_a_line` now ends its line with a
`<br>`, because the bookkeeping runs only where a line ends. Its assertions are unchanged.

The other `sync_clones` survivors are equivalent:

- `len <= self.cur.len()` replaced by `true` is equivalent: `cur` only shrinks in `flush`,
  which resets `clones_len` to `None`, so the guard never sees a longer `len`.
- Replacing the guard with `false`, and `<=` with `>`, only rebuilds the open-clone list
  from scratch, which gives the same result. After the fix the list is synced once per
  line end, so the cost is the same as well.
- The `clone_reserve` mutants went away with the function.

### Item 5: `step_align`

- `base / 2.0` replaced by `base % 2.0` is **killed** by `form-native-range-step-base`
  (`min=5 max=10 step=2 value=10` is 9, at 80%) and by a unit assertion.
- `base + steps.floor() * step` replaced by `-`, and `aligned >= min` replaced by `<`, are
  equivalent. Snapped values sit on the `base + n*step` grid, so a snapped value above the
  lowered max (the last grid value at or below the max) is also above the max itself. That
  makes the lowered max indistinguishable from the max.

### Item 8: gradients

- `repeated` (`copies * len` replaced by `+` or `/`) and `average` (`weight` `-` replaced
  by `+`; `total +=` replaced by `-=` or `*=`) are **killed** by the new unit test. That
  test pins the capped fallback for a period too fine to unroll: the length-weighted
  average.
- The test has no Chrome oracle. A Chrome 156 capture (`repeating-linear-gradient(90deg,
  red 0 0.01px, blue 0.01px 0.05px)`) paints pure red at every probed pixel, because it
  samples one stop per pixel and aliases. Our averaged fallback is a documented memory cap,
  not a Chrome match.
- `copies = to - from` replaced by `+` is equivalent: the copy count only feeds the
  overflow check, and `to + from` stays far below the cap at every period that reaches it.
- `period > 0.0` replaced by `>=` is equivalent: a zero period makes `copies` non-finite,
  and the cap check then sends it to the average either way.
- `k < to` replaced by `<=` is equivalent: it adds one copy past the line, and
  `cut_to_line` drops it.

### Item 9: real bug, fixed test-first

The `contained_flow_baseline` mutants `||` replaced by `&&`, and `- f.y` replaced by
`+ f.y`, both survived. Chrome 156 (`contain-flow-baselines`) puts every inline-flex /
inline-grid on its first item's baseline: the item's first text line, or else its
border-box bottom. That holds with or without size containment. We gave one only to
size-contained ones, and then always the item's bottom.

The fix is `flow_atom_baseline`, which is the old function renamed and generalized. The
`+ f.y` mutant is equivalent (see run 3).

### Item 10

- The `img_src` guard replaced by `true` is **killed**: `src-not-image` covers a span, a
  text input and a div with a `src`, and the HTML-path assertion covers the div.
- `record_img_data_uri` `&&` replaced by `||` is equivalent. It only decodes an extra data
  URI into the image map, and nothing reads that entry, because `img_src` consults the map
  only for `<img>` / `<input type=image>`.

## Re-run on the fixes (run 3)

`--re 'flow_atom_baseline|mark_closing_edges|needs_wrap|step_align|img_src'` on
block.rs, inline.rs, form_paint.rs and boxgen.rs: 107 mutants, 85 caught. The regex
also matched struct-field deletions in unrelated functions (`LayoutBox::relabelled`,
`with_drawn_hyphen`, `flatten_chars`, `close_clones`). Those are outside this audit.

- **`needs_wrap`.** `close_after` `+` replaced by `-` or `*`, and `>` replaced by `>=`, on
  both the HTML and native-metric branches. **Killed** by `inline-close-glued` (live path,
  `end_tags_stay_with_the_atom_before_them_like_chrome_156`), by `inline-close-glued-nested`
  (paint), and by `static_inline_boxes::nested_end_tags_stay_with_the_atom_before_them`,
  each verified by hand.
- **`mark_closing_edges`.** `next.width + next.close_after` replaced by `-` is
  equivalent: consecutive end tags with nothing between them form one closing-only word,
  so a closing-only word never has a `close_after` of its own to add.
- **`flow_atom_baseline`.** `- f.y` replaced by `+ f.y` is equivalent: an atom's fragment
  is laid out at its own origin (`f.y == 0`) before the line places it. The case below
  the page top still passes with the mutation.
- **`step_align`.** The two equivalent survivors from run 1 still survive, as expected.

## Coverage and gates

The full local gate runs before push. It covers fmt and clippy on 1.98.0, the
layout/raster suites with every CI feature set, and tarpaulin floors (layout 99.8, raster
100).
