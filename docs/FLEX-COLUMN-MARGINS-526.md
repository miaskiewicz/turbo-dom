# Signed flex-column intrinsic margins (#526)

Automatic column size must include signed outer contributions. The pinned
Taffy 0.8.3 intrinsic main-size algorithm adds margins and then floors that
outer contribution by the inner flex basis. Negative margins can therefore
inflate the container while its children retain their correct positions.
See [CSS Flexbox intrinsic main sizes](https://drafts.csswg.org/css-flexbox-1/#intrinsic-main-sizes).

The native adapter measures the indefinite column without vertical margins,
adds the signed margins in outer space, applies the original container min/max
constraints and runs the ordinary flex solver with that internal used size.
It restores solver styles and clears affected caches around both passes.
This changes intrinsic sizing, not the painted rectangle after placement.
Hidden and out-of-flow children do not contribute.

Allocated flex geometry and the CSS percentage-height basis are separate.
An automatic column item's used height can remain indefinite to its percentage
children, while a definite container or definite flex basis exposes its used
height, including zero. Positioned containing blocks retain their real used
geometry. Unresolved percentage bases use content; explicit content bases ignore
preferred height during intrinsic measurement but retain its automatic minimum
suggestion. Numeric flex shorthand expands to a zero-percent basis.

## Verification

`flex_column_margins_chrome.rs` checks all parent and child rectangles in 74
text-free, offline Chromium 159.0.8154.6 controls. Fixtures pin collector and case
hashes. Cases cover signed and percentage margins, grow/shrink, content and
percentage bases, min/max, frames, reverse/wrapped columns, nested percentage
heights, definite zero, and hidden/positioned children. The same test passes
without default features. Bypassing the intrinsic correction produces assertion
failures; restoring it passes.

Local validation with Rust 1.98.0:

- Full layout tests with `svg,perf,test-support`: 1,233 passed, zero ignored.
- Strict all-target Clippy, formatting, and changed-source complexity <= 9 pass.
- The frozen partner-logo component has zero measured geometry differences at
  1440, 768 and 390 CSS pixels, including its outer container. Pixel residuals
  remain 1.91%, 3.60% and 3.29%; geometry agreement is not a pixel-parity claim.

The existing legacy paged-layout height policy remains separate from explicit
fractional CSS geometry's definite-zero handling.
