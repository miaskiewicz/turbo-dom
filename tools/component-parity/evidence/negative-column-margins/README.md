# Negative outer margins in auto-height flex columns

Known-red independent reduction for [#526](https://github.com/miaskiewicz/turbo-dom/issues/526), isolated while validating #522. This is separate from the corrected logo row placement and intrinsic image sizing.

`negative-column-margins.html` has no text, images or network dependencies. Chromium 159.0.8154.6 at 390x1000 gives a 457px enclosing height; release source `50ab570d` gives 607px. All four child rectangles agree exactly. The adjacent JSON files retain the browser and native measurements; native output is historical evidence, not a correct expected value.

The root cause and required semantic controls are in #526. A fix must preserve indefinite percentage heights and flex sizing, rather than clipping or replacing the final painted height.

For independent browser reproduction, load the HTML offline at 390x1000 and measure `.stack` and each `.item` with `getBoundingClientRect()`. For native reproduction, pass the absolute fixture path, viewport and selectors `[".stack", ".item"]` to the existing `component_parity` example (the request contract is documented in the component harness README).
