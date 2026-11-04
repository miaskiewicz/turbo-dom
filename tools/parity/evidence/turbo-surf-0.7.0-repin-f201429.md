# turbo-surf repin to turbo-dom f201429: Chrome 156 evidence

Recorded by the turbo-surf release/0.7.0 adoption (surf #127), frozen Chrome 156.0.8078.47.

Measured with a real turbo-surf napi addon at release/0.7.0 3eb9682 (turbo-dom f201429) against b-0.7.0 (turbo-dom e425311), same host, frozen Chrome 156.0.8078.47 for the checks; no structural regression. Static PNGs move toward Chrome: extra-percent-height now pixel-identical to Chrome (was a 25 px band off), conformance-62 overflow clip (34k -> 242 px from Chrome), border-cascade / react-* buttons take Chrome width (form-control UA padding, #239; surf no longer overrides it), embedded-font-range-* button text. Their SVGs follow. 21 image SVGs change serialization only (repeated tiles emitted as <use>; rasterized old vs new are pixel-identical in Chrome, img-svg-fractional differs only by tile-seam antialiasing). Live declarative_shadow_page: document height 317.9 -> 310.5 and last span y 291.4 -> 282 (Chrome: 305 / 277).
