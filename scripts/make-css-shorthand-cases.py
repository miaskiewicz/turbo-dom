#!/usr/bin/env python3
"""Writes crates/turbo-dom/tests/fixtures/css-shorthand-cases.json (miaskiewicz/turbo-dom#90):
one page whose <script id=scenarios> lists, per CSS shorthand Chrome 156 exposes on
CSSStyleDeclaration, the values css-shorthand-probe.js sets on an element's inline style
(setProperty, then the longhands, getPropertyValue and cssText it reads back), plus
hand-written declaration-block scenarios (cssText parsing, overrides, priorities, removal,
CSS-wide keywords, var()). For every first value the probe also re-assembles the shorthand
from its longhands (all, all but one, one CSS-wide keyword, one !important). Regenerate,
then recapture the control (from the repository root):
  python3 scripts/make-css-shorthand-cases.py
  node crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs \\
    --probe crates/turbo-dom/tests/fixtures/css-shorthand-probe.js \\
    --cases crates/turbo-dom/tests/fixtures/css-shorthand-cases.json \\
    --out crates/turbo-dom/tests/fixtures/css_shorthand_chrome156.json --chrome <Chrome 156>
"""
import json
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "turbo-dom", "tests", "fixtures", "css-shorthand-cases.json")

L = "1px"
BOX = ["1px", "1px 2px", "1px 2px 3px", "1px 2px 3px 4px", "0", "0 auto", "1PX   2px", "1px 1px 1px 1px", "1px 2px 1px 2px", "1px 2px 3px 2px", "-1px 5%", "calc(1px + 2%) 3em", "1px 2px 3px 4px 5px", "red", "1px,2px"]
PAIR_LEN = ["1px", "1px 2px", "1px 1px", "0", "auto 1px", "1px 2px 3px", "5%  6EM"]
LINE = ["1px solid red", "solid", "red", "thin", "dashed 2px", "red dotted thick", "1px", "none", "medium none currentcolor", "1px solid red solid", "2px #00ff00 double", "solid rgb(1, 2, 3) 0", "inset 3px transparent"]
TIMES = ["1s"]

values = {
    # 4-value box shorthands
    "margin": BOX + ["auto"], "padding": BOX[:12] + ["auto"], "inset": BOX + ["auto", "auto 1px"],
    "scroll-margin": ["1px", "1px 2px", "1px 2px 3px", "1px 2px 3px 4px", "0", "1px 1px", "5%"],
    "scroll-padding": ["1px", "1px 2px", "1px 2px 3px", "1px 2px 3px 4px", "auto", "0 5%", "auto auto"],
    "border-width": ["1px", "1px 2px", "1px 2px 3px", "1px 2px 3px 4px", "thin medium", "thick", "0", "1px solid"],
    "border-style": ["solid", "solid dashed", "solid dashed dotted", "solid dashed dotted double", "none", "SOLID", "1px"],
    "border-color": ["red", "red blue", "red blue green", "red blue green #000", "currentcolor", "rgb(1, 2, 3) transparent", "#abc", "red 1px"],
    "border-radius": ["1px", "1px 2px", "1px 2px 3px", "1px 2px 3px 4px", "1px / 2px", "1px 2px / 3px 4px 5px", "1px 2px 3px 4px / 5px 6px 7px 8px", "50%", "1px 1px / 1px", "0", "1px /", "/ 1px"],
    "-webkit-border-radius": ["1px", "1px 2px", "1px / 2px", "1px 2px 3px 4px"],
    "corner-shape": ["round", "round scoop", "bevel notch squircle", "round scoop bevel notch", "superellipse(2)", "square square"],
    # 2-value pairs
    "margin-block": PAIR_LEN, "margin-inline": PAIR_LEN, "padding-block": ["1px", "1px 2px", "0", "1px 1px"],
    "padding-inline": ["1px", "1px 2px", "0", "5% 5%"], "inset-block": PAIR_LEN, "inset-inline": PAIR_LEN,
    "scroll-margin-block": ["1px", "1px 2px", "0"], "scroll-margin-inline": ["1px", "1px 2px", "1px 1px"],
    "scroll-padding-block": ["1px", "auto 2px", "0"], "scroll-padding-inline": ["1px", "1px 2px", "auto auto"],
    "border-block-width": ["1px", "1px 2px", "thin thin"], "border-block-style": ["solid", "solid dashed"], "border-block-color": ["red", "red blue", "red red"],
    "border-inline-width": ["1px", "1px thick"], "border-inline-style": ["solid", "none dotted"], "border-inline-color": ["red", "red blue"],
    "gap": ["1px", "1px 2px", "normal", "normal 1px", "1px 1px", "5% 2em", "1px 2px 3px", "-1px"],
    "grid-gap": ["1px", "1px 2px", "normal"],
    "overflow": ["hidden", "hidden auto", "auto auto", "visible clip", "scroll", "HIDDEN", "hidden auto scroll", "overlay"],
    "overscroll-behavior": ["contain", "auto none", "none none", "chain"],
    "contain-intrinsic-size": ["1px", "1px 2px", "auto 1px", "auto 1px auto 2px", "none", "auto none", "1px 1px", "auto 1px 2px"],
    "place-content": ["center", "center start", "space-between", "normal", "start end", "baseline", "first baseline", "last baseline center", "stretch space-around", "left"],
    "place-items": ["center", "center start", "normal", "baseline", "stretch legacy", "start legacy left", "anchor-center", "auto", "first baseline end"],
    "place-self": ["center", "center start", "auto", "auto auto", "normal", "baseline center", "anchor-center"],
    "border-spacing": ["1px", "1px 2px", "0", "1px 1px", "-1px", "1px 2px 3px"],
    "background-position": ["center", "left top", "10px 20px", "right 10px bottom 20px", "50%", "top", "left", "top left", "center center", "10% 20%, 30% 40%", "bottom 10px right", "x"],
    "mask-position": ["center", "10px 20px", "left top, right bottom"],
    "-webkit-mask-position": ["center", "left 10px top"],
    "animation-range": ["normal", "cover", "cover 10% contain 90%", "entry exit", "10% 90%", "normal normal", "cover 0% cover 100%", "entry 10%"],
    "timeline-trigger-activation-range": ["normal", "cover", "entry 10% exit 90%"],
    "timeline-trigger-active-range": ["normal", "auto", "contain"],
    "interest-delay": ["1s", "1s 2s", "normal", "normal 1s", "1s 1s"],
    "column-rule-inset": ["1px", "1px 2px", "overlap-join", "1px 2px 3px 4px"],
    "column-rule-inset-cap": ["1px", "1px 2px"], "column-rule-inset-junction": ["1px", "overlap-join 1px"],
    "column-rule-inset-start": ["1px", "1px 2px"], "column-rule-inset-end": ["1px", "1px 2px"],
    "row-rule-inset": ["1px", "1px 2px"], "row-rule-inset-cap": ["1px"], "row-rule-inset-junction": ["1px"], "row-rule-inset-start": ["1px"], "row-rule-inset-end": ["1px 2px"],
    "rule-inset": ["1px", "1px 2px"], "rule-inset-cap": ["1px"], "rule-inset-junction": ["1px"], "rule-inset-start": ["1px"], "rule-inset-end": ["1px"],
    "rule-break": ["normal", "none", "intersection"], "rule-color": ["red", "red blue"], "rule-style": ["solid", "solid dashed"],
    "rule-width": ["1px", "1px 2px"], "rule-visibility-items": ["all", "around"],
    "corner-top-shape": ["round", "round scoop"], "corner-right-shape": ["bevel"], "corner-bottom-shape": ["notch squircle"], "corner-left-shape": ["square"],
    "corner-block-start-shape": ["round"], "corner-block-end-shape": ["round scoop"], "corner-inline-start-shape": ["bevel"], "corner-inline-end-shape": ["notch"],
    # || line shorthands
    "border": LINE + ["1px solid red / 2px"], "border-top": LINE, "border-right": ["1px solid red", "solid", "red 2px"],
    "border-bottom": ["1px solid red", "dashed"], "border-left": ["1px solid red", "thin"],
    "border-block": ["1px solid red", "solid", "red"], "border-inline": ["1px solid red", "dashed 2px"],
    "border-block-start": ["1px solid red", "solid"], "border-block-end": ["1px solid red", "red"],
    "border-inline-start": ["1px solid red", "2px"], "border-inline-end": ["1px solid red", "dotted"],
    "-webkit-border-before": ["1px solid red"], "-webkit-border-after": ["solid"], "-webkit-border-start": ["red 1px"], "-webkit-border-end": ["1px"],
    "outline": ["1px solid red", "solid", "red", "auto", "auto 1px", "none", "thick dashed", "1px solid red solid", "invert"],
    "column-rule": ["1px solid red", "solid", "red", "thin dotted"], "-webkit-column-rule": ["1px solid red"],
    "row-rule": ["1px solid red", "dashed"], "rule": ["1px solid red", "solid"],
    "text-decoration": ["underline", "underline red", "underline dotted red", "underline 2px", "none", "line-through overline", "red", "underline wavy blue 3px", "auto underline", "from-font", "blink"],
    "text-emphasis": ["dot", "dot red", "filled circle", "open", "'x'", "none", "red", "red sesame"],
    "-webkit-text-emphasis": ["dot red"],
    "-webkit-text-stroke": ["1px red", "1px", "red", "thin"],
    "flex-flow": ["row wrap", "column", "wrap", "row nowrap", "column-reverse wrap-reverse", "nowrap row", "wrap wrap"],
    "-webkit-flex-flow": ["column wrap"],
    "list-style": ["square", "inside", "square inside", "none", "none none", "url(a.png)", "inside url(a.png) circle", "outside", "disc outside none", "'-'", "decimal-leading-zero", "none square", "none inside"],
    "columns": ["2", "10px", "2 10px", "auto", "auto auto", "10px 2", "3 auto", "2 / 10px", "auto 2"],
    "-webkit-columns": ["2 10px"],
    "container": ["a", "a / size", "a b / inline-size", "none", "none / size", "a / normal"],
    "text-wrap": ["balance", "wrap", "nowrap", "pretty", "wrap balance", "nowrap stable", "auto"],
    "white-space": ["pre", "nowrap", "normal", "pre-wrap", "pre-line", "break-spaces", "collapse", "preserve nowrap", "preserve-breaks wrap", "break-spaces nowrap"],
    "font-synthesis": ["none", "weight", "style", "small-caps", "weight style", "weight style small-caps", "style weight"],
    "text-box": ["normal", "trim-both", "cap alphabetic", "trim-start cap", "auto", "trim-both text"],
    "position-try": ["--a", "--a, --b", "most-width --a", "none", "normal --a", "flip-block"],
    "scroll-timeline": ["--a", "--a x", "none", "--a block", "--a, --b y"],
    "view-timeline": ["--a", "--a x", "--a 10px", "--a block auto 10px", "none"],
    "marker": ["url(#m)", "none", "url(a.svg#m)"],
    "font-variant": ["small-caps", "normal", "none", "common-ligatures tabular-nums", "small-caps slashed-zero", "all-small-caps sub", "jis78", "historical-forms", "emoji"],
    "flex": ["1", "auto", "none", "initial", "2 3", "1 0px", "10px", "0", "1 2 30%", "30% 1 2", "content", "0 0", "1.5", "1 1 auto", "-1", "1 2 3", "1px 2px", "0 1 auto"],
    "-webkit-flex": ["1", "auto", "2 3 10px"],
    "font": ["12px Arial", "bold italic 12px/1.5 'Times New Roman', serif", "italic small-caps bold condensed 16px/2 cursive", "caption", "menu", "message-box", "small-caption", "status-bar", "icon",
             "12px", "Arial", "normal 12px Arial", "normal normal normal 12px/normal Arial", "300 12px Arial", "oblique 12px Arial", "oblique 10deg 12px Arial", "ultra-expanded 12px a", "x-large/110% \"New Century Schoolbook\", serif", "12px/normal sans-serif", "small-caps 12px a", "all-small-caps 12px a"],
    "grid-area": ["a", "a / b", "a / b / c", "a / b / c / d", "1", "1 / 3", "1 / 2 / 3 / 4", "span 2", "auto", "auto / auto", "2 a", "span a / 3", "1 / span 2 / a"],
    "grid-row": ["1", "1 / 3", "a", "span 2", "auto", "a / b", "1 / auto", "span 2 / 3"],
    "grid-column": ["1", "1 / 3", "a", "span 2 a", "auto / 3"],
    "grid-template": ["none", "auto / 1fr 1fr", "100px 1fr / 50px 1fr", "\"a b\" \"c d\"", "\"a\" 10px \"b\" 20px / 1fr", "[x] \"a\" 10px [y] / auto", "repeat(2, 1fr) / auto", "subgrid / auto"],
    "grid": ["none", "auto / 1fr 1fr", "\"a b\"", "auto-flow / 1fr", "auto-flow 10px / 1fr", "1fr / auto-flow dense 20px", "100px / auto-flow", "row dense / 1fr", "repeat(3, 1fr) / auto"],
    "border-image": ["url(a.png) 30 round", "none", "url(a.png)", "url(a.png) 10% / 2px / 1px repeat", "linear-gradient(red, blue) 27 fill", "30 / 2", "stretch", "url(a.png) 10 20 30 40 / 1 2 3 4 / 5 6 7 8 space round", "url(a.png) 1 / / 2"],
    "-webkit-mask-box-image": ["url(a.png) 30 round", "none", "url(a.png)"],
    "offset": ["10px 30px", "none", "path('M 0 0 L 10 10')", "path('M 0 0') 10px", "path('M 0 0') 10px auto 45deg", "auto", "left top / center", "ray(45deg) 20% reverse", "path('M0 0') / 10px 20px", "normal"],
    "background": ["red", "url(a.png) no-repeat center / cover red", "none", "url(a.png)", "url(a.png), url(b.png)", "url(a.png) red, url(b.png)", "repeat-x", "fixed", "content-box", "padding-box content-box",
                   "linear-gradient(red, blue)", "10px 20px", "left top / 10px 20px", "transparent", "url(a.png) 0 0 / auto repeat scroll padding-box border-box", "red url(a.png)", "border-box", "text", "space round",
                   "no-repeat repeat"],
    "mask": ["url(a.png)", "none", "url(a.png) no-repeat center / contain", "url(a.png) luminance", "url(a.png) exclude", "url(a.png), url(b.png) subtract", "linear-gradient(red, blue) content-box padding-box"],
    "-webkit-mask": ["url(a.png)", "url(a.png) no-repeat"],
    "transition": ["opacity 1s", "all 1s ease 0s", "none", "opacity 1s, transform 2s linear", "1s", "opacity", "opacity 1s 2s", "opacity 1s steps(4) 2s", "opacity 1s allow-discrete", "all", "0s", "ease-in", "opacity 1s cubic-bezier(0.1, 0.2, 0.3, 0.4)", "none, opacity", "--x 1s"],
    "-webkit-transition": ["opacity 1s"],
    "animation": ["spin 1s infinite", "spin", "1s", "none", "spin 1s ease-in 2s 3 reverse both paused", "spin 1s, fade 2s", "infinite", "spin 1s linear", "1s 2s", "spin scroll()", "a b", "paused", "spin 1s 2s 3 normal forwards running"],
    "-webkit-animation": ["spin 1s infinite"],
    "timeline-trigger": ["none", "--t view()", "--t view() entry 10% exit 90%"],
}


# Round two: more spellings of the shorthands pages use most, valid and invalid.
more = {
    "margin": ["+1px", ".5em", "1.50px", "1e1px", "-0", "1px auto 2px", "auto auto auto auto", "1q", "1px !important", "calc(10px)", "calc(1px + 2px)", "calc(50% - 10px) auto", "min(1px, 2%)", "1px/2px", "0px 0 0px 0"],
    "padding": ["0 0 0 1px", "1em 2em 3em", "-1px", "5% 10%", "calc(100% - 2px)"],
    "border": ["1px solid", "solid red", "#000 1px solid", "1px solid currentColor", "1px solid RED", "thin thick", "1px dashed rgba(0,0,0,.5)", "0 none", "none 0", "1px hsl(120, 100%, 50%) solid", "medium", "1px solid transparent", "solid 1px #fff"],
    "border-top": ["0", "1px solid #ccc", "none"],
    "border-left": ["4px solid #0000ff", "inherit"],
    "border-radius": ["4px", "50% / 10%", "1px 2px 3px 4px / 1px", "0 0 4px 4px", "4PX"],
    "outline": ["0", "2px dotted", "1px solid -webkit-focus-ring-color", "none 0"],
    "background": ["#fff", "rgb(255, 0, 0)", "url(\"a b.png\") center", "url(a.png) center/50%", "url(a.png) 50% 50% / 100% auto no-repeat", "center / contain no-repeat url(a.png)", "none transparent", "#fff url(a.png) repeat-y right top", "url(a.png) top left / 10px", "linear-gradient(to right, red 0%, blue 100%)", "red none", "url(a.png) fixed center", "border-box red", "url(a.png) padding-box", "url(a.png) no-repeat, red", "red, url(a.png)"],
    "font": ["bold 14px Arial", "14px/20px Arial, sans-serif", "italic 1em serif", "700 16px \"Helvetica Neue\", Helvetica", "12px", "bold", "small-caps bold 1rem/1.2 system-ui", "normal normal 400 16px/24px Roboto", "12px Arial bold", "16px -apple-system, BlinkMacSystemFont", "inherit 12px", "50% serif", "smaller monospace", "bolder 12px a", "condensed bold 12px a", "12px/1 a b c"],
    "flex": ["1 1 0", "1 1 0px", "2", "0 0 100px", "1 auto", "auto 1", "1 0 auto", "0 0 0", "1 1 calc(50% - 10px)", "none 1", "fit-content", "max-content 1", "1 1 1 1", "auto auto"],
    "transition": ["all 0.3s ease", "opacity 300ms, transform 300ms", "color .2s ease-in-out", "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1) 0ms", "all 0s", "width 1s 2s ease", "1s opacity", "opacity 1s 2s 3s", "opacity 1s, none", "background-color 150ms linear 0s", "-1s", "opacity 1s -1s"],
    "animation": ["spin 1s linear infinite", "fade 300ms ease-out forwards", "2s spin", "spin 1s 2", "spin infinite 1s", "none 1s", "1s ease-in-out 0s infinite alternate both running pulse", "a 1s, b 2s, c 3s", "spin 1s -2s"],
    "gap": ["0", "1rem 2rem", "normal normal"],
    "inset": ["0 auto auto 0", "-10px"],
    "overflow": ["auto hidden", "clip clip", "hidden visible"],
    "text-decoration": ["underline solid", "none!important", "line-through dashed", "overline underline", "underline currentcolor", "underline 0.1em solid rgb(0, 0, 0)", "underline auto"],
    "list-style": ["disc", "none outside", "square url(a.png)", "inside none", "upper-roman inside"],
    "place-items": ["start", "end center", "self-start", "flex-end flex-start", "safe center", "center safe start"],
    "place-content": ["space-around", "space-evenly center", "center space-between", "end", "flex-start flex-end", "safe center unsafe end"],
    "place-self": ["end", "start center", "stretch", "self-end self-start"],
    "grid-area": ["1 / 1 / 2 / 2", "a / a / a / a", "auto / auto / auto / auto", "span 2 / span 3", "-1", "1 / -1"],
    "grid-column": ["1 / -1", "span 2", "2 / span 3", "a / a", "auto"],
    "grid-row": ["2 / 4", "1 / span 2"],
    "grid-template": ["1fr / 1fr", "auto 1fr / minmax(100px, 1fr) 2fr", "repeat(auto-fill, minmax(100px, 1fr)) / auto"],
    "grid": ["auto-flow dense / 1fr", "1fr 1fr / auto-flow 100px", "\"a\" 1fr / 1fr"],
    "columns": ["3 100px", "100px auto"],
    "flex-flow": ["column nowrap", "row-reverse"],
    "border-width": ["1px 0", "medium thin"],
    "border-color": ["transparent #f00", "currentColor"],
    "border-style": ["none solid", "dashed none dashed"],
    "margin-inline": ["auto", "0 auto"],
    "padding-block": ["4px 8px"],
    "text-emphasis": ["filled", "open dot red"],
    "white-space": ["NOWRAP", "normal nowrap"],
    "border-spacing": ["2px 4px"],
    "background-position": ["right 10% top", "center 10px", "10px center", "left 10px top 20px"],
    "border-image": ["url(a.png) 30 30 stretch", "url(a.png) 30 / 10px", "url(a.png) 30 fill / 10px / 5px round stretch"],
    "container": ["sidebar / inline-size"],
    "contain-intrinsic-size": ["auto 100px auto 200px"],
    "scroll-margin": ["10px 0"],
    "font-variant": ["small-caps tabular-nums", "lining-nums tabular-nums", "no-common-ligatures no-contextual"],
}
for name, vals in more.items():
    if vals:
        values[name] = values[name] + [v for v in vals if v not in values[name]]


# Round three: tokenizer and grammar edge cases (comments, escapes, exponents, nested
# functions, every slash / layer form).
edge = {
    "margin": ["1px/**/2px", "1e3px", "1e-7px", "1.5e6px", "12345678px", "0.000001px", "-.5px", "1px\\9", "1px 2px;", "1px)", "(1px)", "calc(1px - 2px)", "calc(1px + 2px + 3%)", "calc(1px * 2)", "calc(2% - 1%)", "max(1px, 2px)", "calc(1s)", "+0", "0.0", "1PX 2Em 3REM 4vw"],
    "font": ["12px \\41 rial", "12px 'a\\\"b'", "12px \"a\\\nb\"", "12px a, 'b', c d", "12px a,", "12px , a", "12px inherit", "12px a / 2", "bold 900 12px a", "italic italic 12px a", "1001 12px a", "0 12px a", "12px/", "oblique 0deg 12px a", "oblique 1turn 12px a", "condensed 50% a"],
    "background": ["url( a.png )", "url('a b.png')", "url(a b.png)", "url(\"a.png\" x)", "url()", "image-set(url(a.png) 1x)", "red red", "url(a.png) url(b.png)", "center / auto", "center / 10px 20px", "/ 10px", "url(a.png) center /", "repeat-x repeat-y", "padding-box text", "text padding-box", "fixed scroll", "#ff000080", "#f008", "#ggg", "#12345", "hsl(120 100% 50%)", "rgb(1 2 3 / 50%)", "lab(50% 10 10)", "currentColor", "Canvas"],
    "column-rule-inset": ["1px / 2px", "1px 2px / 3px 4px", "1px / 2px / 3px", "overlap-join / 1px"],
    "row-rule-inset": ["1px / 2px 3px"],
    "rule-inset": ["1px / 2px"],
    "container": ["a / size scroll-state", "a / size size", "a / bogus", "a and", "a b c", "none none", "/ size"],
    "offset": ["path('M 0 0') 45deg", "path('M 0 0') 45deg auto", "path('M 0 0') reverse 10deg", "path('M0,0L10-10') 10px", "left top path('M 0 0')", "10px 20px 30px", "bogus", "path('M 0 0') 10px 20px", "none 10px"],
    "timeline-trigger": ["--t view() entry 10% exit 90% / cover", "--t view() cover / normal contain", "--t auto", "--t scroll() cover / cover / cover", "t view()", "--t bogus"],
    "transition": ["opacity 1s, transform", "opacity 1ms", "opacity 0ms 1s", "opacity 1s steps(4, jump-end)", "opacity linear(0, 1)", "opacity 1s ease 2s normal", "all none", "1s 2s 3s", "opacity calc(1s + 1s)"],
    "animation": ["'spin' 1s", "spin 1s 2.5", "spin -1", "spin 1s infinite infinite", "spin alternate-reverse backwards", "1s auto"],
    "grid-template": ["\"a\" [x] [y] 10px", "[a] \"a\" [b] [c] \"b\"", "\"a\" minmax(10px, 1fr)", "\"a\" fit-content(10px)", "\"a\" 1fr \"b\"", "\"a\" auto / repeat(2, 1fr)", "\"a\" / bogus", "10px", "auto / auto / auto", "subgrid [a] / auto", "0 / 0"],
    "grid": ["auto-flow / auto-flow", "dense auto-flow 10px / 1fr", "1fr / dense auto-flow", "auto-flow 10px 20px / 1fr", "auto-flow bogus / 1fr", "1fr / auto-flow bogus", "dense / 1fr"],
    "grid-area": ["span", "span span", "0", "a b", "1 2", "span -1", "a / b / c / d / e", "auto a"],
    "border-image": ["url(a.png) 10 fill 20", "url(a.png) fill 10", "url(a.png) 10 / auto / 1px", "url(a.png) 10 / / ", "url(a.png) 10 / 1 2 3 4 5", "url(a.png) repeat repeat", "url(a.png) round space round", "url(a.png) 1 2 3 4 5", "none none", "10 / 1 / -1"],
    "list-style": ["none none none", "inside inside", "url(a.png) url(b.png)", "'x' 'y'"],
    "text-decoration": ["underline underline", "underline overline underline", "none underline"],
    "text-emphasis": ["filled open", "dot circle", "filled dot red", "'a' 'b'"],
    "place-content": ["safe", "unsafe end", "baseline baseline", "normal bogus", "safe left"],
    "place-items": ["legacy", "center legacy center", "legacy right", "center legacy bogus", "unsafe end safe left"],
    "place-self": ["safe end safe start", "center safe left", "auto bogus"],
    "position-try": ["most-width", "--a, flip-block flip-inline", "--a --b", "flip-block flip-block", "--a,", "most-height none"],
    "view-timeline": ["--a, --b x", "--a 10px 20px", "--a auto auto", "--a x x", "--a bogus", "bogus"],
    "scroll-timeline": ["--a x y", "--a bogus", "bogus"],
    "contain-intrinsic-size": ["auto", "auto auto", "1px 2px 3px", "auto 1px auto"],
    "text-box": ["trim-start trim-end", "cap", "text text", "trim-end ex alphabetic", "auto auto"],
    "font-variant": ["common-ligatures common-ligatures", "stylistic(a) small-caps", "jis78 full-width ruby", "lining-nums oldstyle-nums"],
    "flex": ["1 1", "1 auto 1", "calc(1)", "1 1 1px 1", "content 1 1"],
    "border-radius": ["1px / 2px / 3px", "1px 2px 3px 4px 5px", "-1px"],
    "-webkit-border-radius": ["1px 2px 3px"],
    "columns": ["2 / 10px / 1px", "2 / bogus", "1px 2px", "2 3", "0", "auto / auto"],
    "white-space": ["preserve preserve", "nowrap wrap", "pre nowrap"],
    "font-synthesis": ["weight weight", "bogus"],
    "animation-range": ["entry 10% exit", "normal 10%", "10% bogus", "cover 0%", "cover 100%"],
    "mask": ["url(a.png) add, url(b.png)", "url(a.png) border-box border-box", "url(a.png) fill-box", "url(a.png) no-clip", "url(a.png) / 10px", "url(a.png) center / 50%", "url(a.png) alpha luminance"],
    "-webkit-mask-box-image": ["url(a.png) 10 / 1px", "none 10 round"],
}
edge["margin"] += ["calc(2 * 1px)", "calc(10px / 2)", "calc(1px * 2px)", "calc(1px / 0)", "clamp(1px, 5px, 3px)", "min(1px, 2%)", "min(1px)", "calc(1px 2px)"]
edge["transition"] += ["opacity linear(0, 0.5, 1)", "opacity linear(0, 0.25 50%, 0.75, 1)", "opacity linear(0, 1 20% 40%)", "opacity linear(1)", "opacity steps(1, start)"]
edge["background"] += ["rgb(10%, 20%, 30%)", "rgba(1, 2, 3, 0.25)", "hsl(120, 100%, 25%)", "hsla(0, 100%, 50%, .5)", "hsl(1turn 50% 50%)", "rgb(1, 2 3)", "oklab(40% 0.1 0.1)", "#ff0000ff"]
for name, vals in edge.items():
    values[name] = values[name] + [v for v in vals if v not in values[name]]

# Round five: cases a mutation run of rtdom::css_shorthand and src/runtime/css-shorthand.mjs
# showed unpinned (sign and range checks, number spelling, tokenizer escapes / comments / bad
# tokens, calc units and operators, hsl() / rgb() forms, linear() / steps(), positions, repeats,
# boxes, font weight / style / stretch / line-height).
mutation = {
    "margin": ["0.00001px", "0.0001px", "100000px", "999999px", "1000000px", "1234567px", "0.1234567px", "-0.00001px",
               "1px/* c */", "/* c */1px", "1px /* open", "1px !2px", "1px !", "1px]", "1px}", "[1px]",
               "calc(1mm + 1px)", "calc(1pt + 1px)", "calc(1pc + 1px)", "calc(1cm + 1px)", "calc(1px / 2px)", "calc(2px * 3)",
               "calc(3 * 2px)", "calc(1px * 2 * 3)", "calc(1px + 2% - 3em)", "calc(-1px)", "calc(1% + 1px - 1%)", "calc(1px / 2 + 1px)",
               "calc(2 / 1px)", "calc(1em + 1px + 1%)", "calc(1px + 1)", "clamp(1px, 2px)", "max(1px, 2px, 3px)", "min(1px, 2em)",
               "calc(infinity * 1px)", "calc(-infinity * 1px)", "calc(NaN * 1px)", "calc((1px + 2px) * 2)", "calc(1px - -1px)",
               "calc(1px +)", "calc()", "calc(1px, 2px)", "clamp(1px, 2px, 3px, 4px)", "calc(1px + 1s)"],
    "padding": ["-1px 0", "calc(-1px)", "0 -5%"],
    "border-width": ["-1px", "1px -1px"], "scroll-padding": ["-1px", "auto -1px"], "scroll-margin": ["-1px", "-1px 2px"],
    "gap": ["1px -1px", "-5%"], "border-spacing": ["1px -1px"], "border-radius": ["1px -1px", "1px / -1px"],
    "outline": ["-1px solid", "solid -1px red"], "column-rule": ["-1px solid red"], "-webkit-text-stroke": ["-1px red", "-1px"],
    "text-decoration": ["underline -1px", "underline -10%", "underline 10%", "underline from-font"],
    "columns": ["-1px", "0 2", "-2", "2 -1px", "1px 0", "1.5", "0"],
    "flex": ["-1px", "1 -1", "1 1 -1px", "1 1 -10%", "-1 1"],
    "font": ["-1px a", "12px/-1 a", "12px/-1px a", "12px/-10% a", "1000 12px a", "1 12px a", "0.5 12px a", "999.5 12px a",
             "oblique -90deg 12px a", "oblique 90deg 12px a", "oblique 91deg 12px a", "oblique -10deg 12px a", "oblique 14deg 12px a",
             "oblique 0.5turn 12px a", "12px/0 a", "50% a", "-10% a", "12px \"a\\\\b\"", "12px a\\\\b", "12px \\61", "12px \\0 a",
             "12px \\110000 a", "12px \\31 a", "12px -\\31 a", "12px --a", "12px -a", "12px \"a\\\"\"", "12px a\\ b", "12px 'a",
             "12px \"a\\", "12px a\\", "12px \"a\nb\"", "12px #a", "12px a\\\\"],
    "background": ["center / -1px", "-10px -20px", "left -10px top -5%", "url(a.png) center / 10px -1px", "url(a\\)b.png)",
                   "url(a\\ b.png)", "url(  a.png  )", "url(a.png ", "url(a.png", "url(a\"b.png)", "url(a\\\nb.png)",
                   "hsl(60, 100%, 50%)", "hsl(180, 100%, 50%)", "hsl(240, 100%, 50%)", "hsl(300, 100%, 50%)", "hsl(0, 0%, 50%)",
                   "hsl(30deg 50% 75%)", "hsl(0.5turn 50% 50%)", "hsl(100grad 50% 50%)", "hsl(1rad 50% 50%)", "hsl(120, 100%)",
                   "hsl(120 100% 50% / 50%)", "hsla(120, 100%, 50%)", "hsl(120, 100%, 50%, 0.5)", "hsl(120 100% 50% /)",
                   "hsl(120, 100% 50%)", "hsl(-120, 100%, 50%)", "hsl(480, 100%, 50%)", "hsl(120, 100%, 80%)", "hsl(120, 50%, 10%)",
                   "rgb(1 2 3 / 0.5)", "rgb(1, 2, 3, 50%)", "rgb(1 2 3 /)", "rgba(1,2,3)", "rgb(1,2)", "rgb(1, 2, 3, 4, 5)",
                   "left 10px top / 10px", "url(a.png) right 10px bottom", "repeat no-repeat", "repeat repeat", "round space",
                   "space space", "repeat-x repeat", "border-box padding-box", "padding-box padding-box", "content-box border-box",
                   "border-box text", "text text", "border-box border-box", "padding-box", "url(a.png) 10px 20px / auto 50%",
                   "url(a.png) no-repeat, red", "url(a.png) left top", "url(a.png) 0% 0%"],
    "background-position": ["left 10px", "10px top", "top 10px", "center left", "center top 10px", "left center",
                            "right 10px center", "center right 10px", "bottom right", "left 10px bottom", "10px", "top top",
                            "left right", "10px 20px 30px", "center center center", "left 10px top 20px right",
                            "right 10px bottom 20px, center", "-10px -5%", "bottom -10px right -5%", "center bottom", "top center"],
    "mask-position": ["left 10px bottom 5%", "right center", "-1px -1px"],
    "border-image": ["url(a.png) -1", "url(a.png) 10 / -1px", "url(a.png) 10 / 1 / -1px", "url(a.png) 10 / 1 / 1",
                     "url(a.png) 10 / 1px 2 3% auto", "url(a.png) 10% 20 30% 40", "url(a.png) 10 / 1 / 1px 2", "url(a.png) 10 10",
                     "url(a.png) 10 10 10", "url(a.png) 1 2 3 4 fill", "url(a.png) 10 / 1 1"],
    "transition": ["opacity -1s", "opacity 1s -1s", "opacity linear(0, 0.5 25% 75%, 1)", "opacity linear(0 0%, 1 100%)",
                   "opacity linear(0, 1 50% 25%)", "opacity linear(0, 0.5 10% 20% 30%, 1)", "opacity linear()", "opacity linear(a)",
                   "opacity steps(0)", "opacity steps(2, jump-none)", "opacity steps(1, jump-none)", "opacity cubic-bezier(2, 0, 0, 1)",
                   "opacity linear(0, 0.2, 0.4 40%, 1)", "opacity linear(0 20%, 0.5, 1 80%)", "opacity steps(2, end)", "opacity steps(2, start)",
                   "opacity 1s, color", "opacity, color 1s"],
    "animation": ["spin -1s", "spin 1s -1s", "spin 1s 2 -1", "spin 0", "spin 1s 2s infinite", "spin 1s 1.5"],
    "offset": ["path('M 0 0') -10px", "path('M 0 0') -10%", "left -10px top -20px / center", "path('M 0 0') 10px reverse",
               "path('M 0 0') auto", "path('M 0 0') reverse", "auto / left top", "10px 20px / auto"],
    "grid-template": ["-1px / 1px", "1px / -10%", "10% / 1fr", "\"a\" -1px", "\"a\" 10% / 1fr", "repeat(2, -1px) / auto",
                      "minmax(-1px, 1fr) / auto", "10px [a] / auto"],
    "grid": ["auto-flow -1px / 1fr", "1fr / auto-flow -1px", "auto-flow / auto", "dense auto-flow / 1fr", "auto-flow dense 10px / 1fr",
             "1fr / auto-flow", "1fr / dense", "auto-flow 10% / 1fr"],
    "grid-area": ["a / b / c", "1 / a", "span 2 / span 2", "a / 1", "a / a / b", "1 / 2 / 3", "a / b / a"],
    "grid-row": ["a / a", "1 / 1", "span 1 / a"],
    "contain-intrinsic-size": ["-1px", "auto -1px", "auto 1px auto -1px"],
    "view-timeline": ["--a -10px", "--a 10px -10px", "--a auto -10%", "--a 10%"],
    "animation-range": ["cover -10%", "entry -10px exit 10px", "-10% 110%", "contain", "exit 10%", "entry 0% entry 100%", "normal cover"],
    "column-rule-inset": ["-1px", "-1px / -2px", "overlap-join overlap-join"],
    "interest-delay": ["-1s", "1s -1s", "normal normal"],
    "list-style": ["square inside url(a.png)", "inside square", "none url(a.png)", "url(a.png) none", "linear-gradient(red, blue)"],
    "text-emphasis": ["filled red", "red open", "'x' red"],
    "font-variant": ["stylistic(a)", "historical-forms small-caps", "styleset(a, b)", "character-variant(a)", "swash(a) ornaments(b)", "annotation(a)"],
    "place-items": ["legacy center", "legacy left", "legacy right center", "right legacy"],
    "place-self": ["left", "right start", "anchor-center end"],
    "flex-flow": ["column-reverse", "wrap-reverse", "row row"],
    "container": ["a b", "a / inline-size", "none / normal"],
    "text-box": ["trim-end", "trim-both cap alphabetic", "trim-start text"],
    "white-space": ["preserve-spaces", "preserve-breaks nowrap", "collapse wrap"],
    "scroll-timeline": ["--a inline", "--a, --b", "none x"],
    "timeline-trigger": ["--t view() normal", "--t scroll() entry", "--t view() / entry"],
    "mask": ["url(a.png) left 10px top", "url(a.png) repeat-x", "url(a.png) round space", "url(a.png) content-box", "url(a.png) border-box padding-box",
             "url(a.png) match-source", "url(a.png) intersect"],
    "position-try": ["most-inline-size --a", "most-block-size --a, --b"],
}
# Round six (second mutation run): positions, url escapes, layer boundaries.
mutation["background-position"] += ["left 10px right", "top 10px center", "center center 10px", "left top 10px", "bottom 10px left 5px"]
mutation["background"] += ["url(a\\62 .png)", "url(a.png )x", "#\\31 23", "url(a.png) center / 10px 20px, url(b.png) 0 0", "url(a.png) 1px 2px / 3px"]
mutation["margin"] += ["calc(1hz + 1px)", "calc(1px * (2 - 1))", "calc(min(1px, 2px) * 2)", "calc(1px + calc(1%))", "calc(pi)", "calc(1px * 2 / 4)"]
mutation["transition"] += ["opacity cubic-bezier(0, 0, 1, 1)", "opacity cubic-bezier(-0.1, 0, 1, 1)", "opacity steps(1, start)", "opacity steps(0, start)", "opacity linear(0, 0.5 20% 10%, 1)"]
mutation["font"] += ["oblique 90deg 12px a", "oblique -90deg 12px a", "oblique 0deg 12px a", "12px \\d800 a", "12px \\10ffff a"]
for name, vals in mutation.items():
    vals = list(dict.fromkeys(vals))
    values[name] = values[name] + [v for v in vals if v not in values[name]]

# Round seven (coverage, P5-cov): the grammar and serializer branches no earlier scenario
# reached -- empty values, slash and list counts past a grammar's limit, calc() / min()
# operands Chrome keeps or rejects, place-* tails, legacy justify-items last.
coverage = {
    "margin": ["/**/", "calc(1em + min(1px, 2%))", "calc(1in + 1em)", "calc((1px 2px))", "calc((1px , 2px))", "calc(1px + abs(1px))",
               "calc(min(1px) * min(2px))", "min(1px 2px)", "min(1px, calc(1px + 1em))",
               "min(1px, 1em + 1px)", "min(1px, min(1px, 1em))", "calc(1px + min(1px, 1em))"],
    "place-content": ["baseline center", "last baseline end", "center start end"],
    "place-items": ["center right legacy", "center start end"],
    "place-self": ["center start end"],
    "background-position": ["left top center"],
    "animation-range": ["10% 20% 30%"],
    "columns": ["/ 10px", "1px 2 auto"],
    "container": ["a /", "a / size / x"],
    "grid-row": ["1 / 2 / 3"],
    "grid-template": ["/ 1fr", "1fr /"],
    "grid": ["1fr /", "/ 1fr"],
    "offset": ["path('M 0 0') 10px 0", "path('M 0 0') / left / top", "path(M 0 0)", "path('M 0 0') 10px auto 0"],
    "transition": ["opacity cubic-bezier(calc(1px), 0, 1, 1)", "opacity cubic-bezier(min(0.5, 1), 0, 1, 1)", "opacity cubic-bezier(0 0, 0, 1, 1)",
                   "opacity linear(calc(0), 1)", "opacity linear(0, 1 calc(50%))", "opacity steps(1, 2, 3)"],
    "background": ["hsl(1px 50% 50%)", "hsl(10% 50% 50%)"],
    "animation": ["forwards none", "none forwards", "spin 1s, none 2s"],
    "font-variant": ["swash(a) bogus(b)"],
}
for name, vals in coverage.items():
    values[name] = values[name] + [v for v in vals if v not in values[name]]

# The names the probe measures with `initial`: Chrome 156's shorthands and property aliases
# (every CSSStyleDeclaration property name whose `initial` sets other or several longhands).
NAMES = ["-webkit-align-content", "-webkit-align-items", "-webkit-align-self", "-webkit-animation", "-webkit-animation-delay", "-webkit-animation-direction", "-webkit-animation-duration", "-webkit-animation-fill-mode", "-webkit-animation-iteration-count", "-webkit-animation-name", "-webkit-animation-play-state", "-webkit-animation-timing-function", "-webkit-app-region", "-webkit-appearance", "-webkit-backface-visibility", "-webkit-background-clip", "-webkit-background-origin", "-webkit-background-size", "-webkit-border-after", "-webkit-border-after-color", "-webkit-border-after-style", "-webkit-border-after-width", "-webkit-border-before", "-webkit-border-before-color", "-webkit-border-before-style", "-webkit-border-before-width", "-webkit-border-bottom-left-radius", "-webkit-border-bottom-right-radius", "-webkit-border-end", "-webkit-border-end-color", "-webkit-border-end-style", "-webkit-border-end-width", "-webkit-border-radius", "-webkit-border-start", "-webkit-border-start-color", "-webkit-border-start-style", "-webkit-border-start-width", "-webkit-border-top-left-radius", "-webkit-border-top-right-radius", "-webkit-box-shadow", "-webkit-box-sizing", "-webkit-clip-path", "-webkit-column-break-after", "-webkit-column-break-before", "-webkit-column-break-inside", "-webkit-column-count", "-webkit-column-gap", "-webkit-column-rule", "-webkit-column-rule-color", "-webkit-column-rule-style", "-webkit-column-rule-width", "-webkit-column-span", "-webkit-column-width", "-webkit-columns", "-webkit-filter", "-webkit-flex", "-webkit-flex-basis", "-webkit-flex-direction", "-webkit-flex-flow", "-webkit-flex-grow", "-webkit-flex-shrink", "-webkit-flex-wrap", "-webkit-font-feature-settings", "-webkit-hyphenate-character", "-webkit-justify-content", "-webkit-logical-height", "-webkit-logical-width", "-webkit-margin-after", "-webkit-margin-before", "-webkit-margin-end", "-webkit-margin-start", "-webkit-mask", "-webkit-mask-box-image", "-webkit-mask-clip", "-webkit-mask-composite", "-webkit-mask-image", "-webkit-mask-origin", "-webkit-mask-position", "-webkit-mask-repeat", "-webkit-mask-size", "-webkit-max-logical-height", "-webkit-max-logical-width", "-webkit-min-logical-height", "-webkit-min-logical-width", "-webkit-opacity", "-webkit-order", "-webkit-padding-after", "-webkit-padding-before", "-webkit-padding-end", "-webkit-padding-start", "-webkit-perspective", "-webkit-perspective-origin", "-webkit-print-color-adjust", "-webkit-shape-image-threshold", "-webkit-shape-margin", "-webkit-shape-outside", "-webkit-text-emphasis", "-webkit-text-emphasis-color", "-webkit-text-emphasis-position", "-webkit-text-emphasis-style", "-webkit-text-size-adjust", "-webkit-text-stroke", "-webkit-transform", "-webkit-transform-origin", "-webkit-transform-style", "-webkit-transition", "-webkit-transition-delay", "-webkit-transition-duration", "-webkit-transition-property", "-webkit-transition-timing-function", "-webkit-user-select", "animation", "animation-range", "background", "background-position", "border", "border-block", "border-block-color", "border-block-end", "border-block-start", "border-block-style", "border-block-width", "border-bottom", "border-color", "border-image", "border-inline", "border-inline-color", "border-inline-end", "border-inline-start", "border-inline-style", "border-inline-width", "border-left", "border-radius", "border-right", "border-spacing", "border-style", "border-top", "border-width", "column-rule", "column-rule-inset", "column-rule-inset-cap", "column-rule-inset-end", "column-rule-inset-junction", "column-rule-inset-start", "columns", "contain-intrinsic-size", "container", "corner-block-end-shape", "corner-block-start-shape", "corner-bottom-shape", "corner-inline-end-shape", "corner-inline-start-shape", "corner-left-shape", "corner-right-shape", "corner-shape", "corner-top-shape", "flex", "flex-flow", "font", "font-synthesis", "font-variant", "gap", "grid", "grid-area", "grid-column", "grid-column-gap", "grid-gap", "grid-row", "grid-row-gap", "grid-template", "inset", "inset-block", "inset-inline", "interest-delay", "list-style", "margin", "margin-block", "margin-inline", "marker", "mask", "mask-position", "offset", "outline", "overflow", "overscroll-behavior", "padding", "padding-block", "padding-inline", "page-break-after", "page-break-before", "page-break-inside", "place-content", "place-items", "place-self", "position-try", "row-rule", "row-rule-inset", "row-rule-inset-cap", "row-rule-inset-end", "row-rule-inset-junction", "row-rule-inset-start", "rule", "rule-break", "rule-color", "rule-inset", "rule-inset-cap", "rule-inset-end", "rule-inset-junction", "rule-inset-start", "rule-style", "rule-visibility-items", "rule-width", "scroll-margin", "scroll-margin-block", "scroll-margin-inline", "scroll-padding", "scroll-padding-block", "scroll-padding-inline", "scroll-timeline", "text-box", "text-decoration", "text-emphasis", "text-wrap", "timeline-trigger", "timeline-trigger-activation-range", "timeline-trigger-active-range", "transition", "view-timeline", "white-space", "word-wrap"]

# Hand-written declaration-block scenarios (each op: css = setAttribute('style'), set =
# setProperty(name, value[, priority]), remove = removeProperty(name), get = getPropertyValue).
blocks = {
    "css-shorthand-then-longhand": [["css", "margin: 1px; margin-top: 2px"]],
    "css-longhands-assemble": [["css", "margin-top: 1px; margin-right: 1px; margin-bottom: 1px; margin-left: 1px"]],
    "css-longhands-interleaved": [["css", "margin-top: 1px; color: red; margin-right: 1px; margin-bottom: 1px; margin-left: 1px"]],
    "css-important-shorthand": [["css", "margin: 1px !important"]],
    "css-important-longhand": [["css", "margin: 1px; margin-top: 2px !important"]],
    "css-important-beats-later": [["css", "margin: 1px !important; margin-top: 2px"]],
    "css-inherit-shorthand": [["css", "margin: inherit"]],
    "css-inherit-then-longhand": [["css", "margin: inherit; margin-top: 1px"]],
    "css-var-shorthand": [["css", "margin: var(--x)"]],
    "css-var-then-longhand": [["css", "margin: var(--x); margin-top: 1px"]],
    "css-var-fallback": [["css", "flex: var(--f, 1)"]],
    "css-duplicate-order": [["css", "color: red; width: 1px; color: blue"]],
    "css-border-sides": [["css", "border-top: 1px solid red; border-right: 1px solid red; border-bottom: 1px solid red; border-left: 1px solid red"]],
    "css-border-parts": [["css", "border-width: 1px; border-style: solid; border-color: red"]],
    "css-border-then-image": [["css", "border: 1px solid red; border-image: url(a.png) 30"]],
    "css-border-then-top": [["css", "border: 1px solid red; border-top-color: blue"]],
    "css-border-then-width": [["css", "border: 1px solid red; border-width: 2px"]],
    "css-border-style-mixed": [["css", "border: 1px solid red; border-left-style: dashed"]],
    "css-background-then-color": [["css", "background: url(a.png); background-color: red"]],
    "css-font-then-size": [["css", "font: 12px Arial; font-size: 14px"]],
    "css-font-then-variant": [["css", "font: 12px Arial; font-variant-numeric: tabular-nums"]],
    "css-font-then-kerning": [["css", "font: 12px Arial; font-kerning: none"]],
    "css-flex-then-basis": [["css", "flex: 1; flex-basis: 10px"]],
    "css-mixed-many": [["css", "display: flex; flex: 1; margin: 0 auto; padding: 4px 8px; border: 1px solid #ccc; color: red"]],
    "css-invalid-shorthand-keeps-others": [["css", "color: red; margin: 1px 2px 3px 4px 5px; width: 1px"]],
    "css-garbage": [["css", "margin: ; flex: ; border: 1px solid red red"]],
    "css-overflow-then-x": [["css", "overflow: hidden; overflow-y: auto"]],
    "css-gap-longhands": [["css", "row-gap: 1px; column-gap: 1px"]],
    "css-inset-longhands": [["css", "top: 0; right: 0; bottom: 0; left: 0"]],
    "css-transition-longhands": [["css", "transition-property: opacity; transition-duration: 1s; transition-timing-function: ease; transition-delay: 0s; transition-behavior: normal"]],
    "css-transition-list-mismatch": [["css", "transition: opacity 1s, color 2s; transition-duration: 1s"]],
    "css-animation-then-name": [["css", "animation: spin 1s; animation-name: fade"]],
    "css-list-style-then-type": [["css", "list-style: square; list-style-type: disc"]],
    "css-text-decoration-then-line": [["css", "text-decoration: underline red; text-decoration-line: overline"]],
    "css-grid-area-longhands": [["css", "grid-row-start: a; grid-column-start: a; grid-row-end: a; grid-column-end: a"]],
    "css-grid-area-mixed": [["css", "grid-row-start: 1; grid-column-start: 2; grid-row-end: auto; grid-column-end: auto"]],
    "css-place-items-longhands": [["css", "align-items: center; justify-items: center"]],
    "css-white-space-longhands": [["css", "white-space-collapse: preserve; text-wrap-mode: wrap"]],
    "set-shorthand-twice": [["set", "margin", "1px"], ["set", "margin", "2px 3px"]],
    "set-longhand-after-shorthand": [["set", "margin", "1px"], ["set", "margin-top", "2px"]],
    "set-shorthand-after-longhand": [["set", "margin-top", "2px"], ["set", "color", "red"], ["set", "margin", "1px"]],
    "set-important": [["set", "padding", "1px", "important"], ["set", "padding-top", "2px"]],
    "set-important-longhand-then-shorthand": [["set", "padding-top", "2px", "important"], ["set", "padding", "1px"]],
    "set-empty-removes": [["set", "margin", "1px"], ["set", "margin", ""]],
    "set-invalid-keeps": [["set", "margin", "1px"], ["set", "margin", "bogus"]],
    "set-bad-priority": [["set", "margin", "1px", "bogus"]],
    "set-camel": [["set", "flex", "1"], ["get", "flex"], ["get", "flex-grow"]],
    "remove-shorthand": [["set", "margin", "1px"], ["set", "margin-top", "2px"], ["remove", "margin"]],
    "remove-shorthand-partial": [["set", "margin-top", "2px"], ["set", "color", "red"], ["remove", "margin"]],
    "remove-longhand": [["set", "margin", "1px"], ["remove", "margin-top"]],
    "remove-flex": [["set", "flex", "1"], ["remove", "flex"]],
    "remove-border": [["set", "border", "1px solid red"], ["remove", "border-top"]],
    "remove-alias": [["set", "-webkit-flex", "1"], ["remove", "-webkit-flex"]],
    "alias-word-wrap": [["set", "word-wrap", "break-word"], ["get", "word-wrap"]],
    "alias-shorthand-get": [["set", "flex", "2"], ["get", "-webkit-flex"]],
    "css-text-roundtrip": [["css", "margin: 1px 2px; flex: 1; border: 1px solid red; font: bold 12px/1.5 Arial; background: url(a.png) red"], ["recss"]],
    # Round five (mutation run): declaration-list parsing, pending var() across shorthands,
    # serializations that depend on several longhands.
    "css-comment-between": [["css", "color: red; /* x */ width: 1px"]],
    "css-comment-unterminated": [["css", "color: red; /* width: 1px"]],
    "css-qualified-rule": [["css", "color: red; a { color: blue } width: 1px"]],
    "css-brackets": [["css", "color: red; x: [a;b]; width: 1px"]],
    "css-braces-value": [["css", "color: red; x: {a;b}; width: 1px"]],
    "css-parens-open": [["css", "width: calc(1px; color: red"]],
    "css-no-colon": [["css", "color red; width: 1px"]],
    "css-close-brackets": [["css", "color: red; ]; width: 1px; }; height: 2px"]],
    "css-empty-name": [["css", ": red; width: 1px"]],
    "css-var-two-shorthands": [["css", "margin: var(--a); padding: var(--a)"]],
    "css-var-shorthand-twice": [["css", "margin: var(--a); margin: var(--b)"]],
    "css-var-then-sibling": [["css", "border: var(--a); border-top: 1px solid red"]],
    "css-var-same-text": [["css", "margin: var(--a); padding: 1px"], ["set", "padding", "var(--a)"]],
    "css-radius-mixed": [["css", "border-top-left-radius: 1px 2px; border-top-right-radius: 1px; border-bottom-right-radius: 1px; border-bottom-left-radius: 1px"]],
    "css-radius-elliptic": [["css", "border-top-left-radius: 1px 2px; border-top-right-radius: 3px 4px; border-bottom-right-radius: 1px; border-bottom-left-radius: 1px 5px"]],
    "css-grid-column-flow": [["css", "grid: 1fr / auto-flow 10px; grid-auto-flow: column"]],
    "css-grid-row-flow": [["css", "grid: auto-flow 10px / 1fr"]],
    "css-grid-dense": [["css", "grid: auto-flow dense / 1fr; grid-auto-columns: 10px"]],
    "css-grid-column-dense": [["css", "grid: 1fr / auto-flow dense; grid-auto-rows: 10px"]],
    "css-grid-flow-cols-set": [["css", "grid: auto-flow / 1fr; grid-template-rows: 10px"]],
    "css-grid-areas-mismatch": [["css", "grid-template: \"a b\" / 1fr; grid-template-rows: 1px 2px"]],
    "css-font-stretch-percent": [["css", "font: 12px a; font-stretch: 87.5%"]],
    "css-font-stretch-odd": [["css", "font: 12px a; font-stretch: 80%"]],
    "css-font-stretch-keyword": [["css", "font: 12px a; font-stretch: condensed"]],
    "css-font-style-oblique": [["css", "font: 12px a; font-style: oblique 14deg"]],
    "css-font-line-height": [["css", "font: 12px a; line-height: 2"]],
    "css-animation-timeline": [["css", "animation: spin 1s; animation-timeline: scroll()"]],
    "css-animation-range": [["css", "animation: spin 1s; animation-range-start: cover"]],
    "css-animation-range-end": [["css", "animation: spin 1s; animation-range-end: contain"]],
    "css-animation-trigger": [["css", "animation: spin 1s; animation-trigger: --t"]],
    "css-animation-lists": [["css", "animation: a 1s, b 2s; animation-duration: 1s, 2s, 3s"]],
    "css-transition-lists": [["css", "transition: opacity 1s, color 2s; transition-delay: 1s, 2s, 3s"]],
    "css-transition-behavior": [["css", "transition: opacity 1s; transition-behavior: allow-discrete"]],
    "css-background-layers": [["css", "background: url(a.png), url(b.png); background-repeat: no-repeat"]],
    "css-background-position-parts": [["css", "background-position-x: left 10px; background-position-y: top"]],
    "css-background-size-only": [["css", "background: red; background-size: 10px"]],
    "css-background-boxes": [["css", "background: red; background-origin: content-box"]],
    "css-background-clip": [["css", "background: red; background-clip: content-box"]],
    "css-background-origin-clip": [["css", "background: red; background-origin: padding-box; background-clip: border-box"]],
    "css-mask-boxes": [["css", "mask: url(a.png); mask-origin: content-box"]],
    "css-mask-layers": [["css", "mask: url(a.png), url(b.png); mask-mode: alpha"]],
    "css-rule-inset-mismatch": [["css", "column-rule-inset: 1px; column-rule-inset-cap-end: 2px"]],
    "css-white-space-pre-wrap": [["css", "white-space-collapse: preserve; text-wrap-mode: wrap"]],
    "css-white-space-preserve-nowrap": [["css", "white-space-collapse: preserve; text-wrap-mode: nowrap"]],
    "css-white-space-collapse-nowrap": [["css", "white-space-collapse: collapse; text-wrap-mode: nowrap"]],
    "css-white-space-breaks": [["css", "white-space-collapse: preserve-breaks; text-wrap-mode: wrap"]],
    "css-white-space-spaces": [["css", "white-space-collapse: break-spaces; text-wrap-mode: wrap"]],
    "css-white-space-other": [["css", "white-space-collapse: preserve-spaces; text-wrap-mode: nowrap"]],
    "css-timeline-trigger-none": [["css", "timeline-trigger: none"]],
    "css-timeline-trigger-mixed": [["css", "timeline-trigger: none; timeline-trigger-source: view()"]],
    "css-position-try-mismatch": [["css", "position-try: --a, --b; position-try-order: most-width"]],
    "css-list-style-none": [["css", "list-style: none; list-style-image: url(a.png)"]],
    "css-text-decoration-line": [["css", "text-decoration: underline; text-decoration-line: none"]],
    "css-text-decoration-thickness": [["css", "text-decoration: underline; text-decoration-thickness: 2px"]],
    "css-columns-width": [["css", "columns: 2; column-width: 10px"]],
    "css-columns-height": [["css", "columns: 2; column-height: 10px"]],
    "css-offset-parts": [["css", "offset: path('M 0 0'); offset-rotate: reverse"]],
    "css-offset-anchor": [["css", "offset: path('M 0 0'); offset-anchor: left top"]],
    "css-offset-position": [["css", "offset: none; offset-position: left top"]],
    "css-flex-parts": [["css", "flex-grow: 2; flex-shrink: 1; flex-basis: 0%"]],
    "css-border-image-parts": [["css", "border: 1px solid red; border-image-slice: 10"]],
    "css-border-image-outset": [["css", "border-image: url(a.png); border-image-outset: 1px"]],
    "css-text-emphasis-parts": [["css", "text-emphasis-style: dot; text-emphasis-color: red"]],
    "css-contain-intrinsic": [["css", "contain-intrinsic-width: auto 1px; contain-intrinsic-height: 2px"]],
    "css-overflow-mixed": [["css", "overflow-x: hidden; overflow-y: auto"]],
    "css-place-legacy": [["css", "align-items: center; justify-items: legacy center"]],
    "css-place-legacy-only": [["css", "align-items: center; justify-items: legacy"]],
    "css-gap-mixed": [["css", "row-gap: 1px; column-gap: normal"]],
}

# Round five, measured one-off against Chrome 156 while fixing it: declaration blocks whose
# cssText pins list lengths, position / origin forms, family and animation names, calc().
PROBED = ["animation: a 1s, b 2s; animation-duration: 1s, 2s, 3s", "animation: a 1s, b 2s, c 3s; animation-duration: 1s", "animation: a 1s, b 2s; animation-name: x, y, z", "animation: a 1s, b 2s; animation-delay: 5s", "transition: opacity 1s, color 2s; transition-delay: 1s, 2s, 3s", "transition: opacity 1s, color 2s, width 3s; transition-delay: 1s", "transition: opacity 1s, color 2s; transition-property: a, b, c", "transition: opacity 1s, color 2s; transition-duration: 0s, 0s, 0s", "background: url(a.png), url(b.png); background-repeat: no-repeat", "background: url(a.png); background-repeat: no-repeat, repeat-x", "background: url(a.png), url(b.png); background-size: 10px", "background: url(a.png), url(b.png); background-position-x: 10px", "background: url(a.png), url(b.png); background-clip: content-box", "background: red; background-size: 10px", "background: url(a.png); background-size: 10px", "background: red; background-position-x: 10px", "background: red; background-position-x: 10px; background-size: 10px", "background: red; background-repeat: no-repeat; background-size: 10px", "background: url(a.png), url(b.png); background-size: 10px, 20px, 30px", "mask: url(a.png), url(b.png); mask-mode: alpha", "mask: url(a.png); mask-mode: alpha, luminance", "mask: url(a.png), url(b.png); mask-size: 10px", "mask: none; mask-size: 10px", "timeline-trigger: none; timeline-trigger-source: view()", "timeline-trigger: none; timeline-trigger-active-range-start: cover", "timeline-trigger: --t; timeline-trigger-active-range-start: cover", "timeline-trigger: none; timeline-trigger-activation-range-start: cover", "timeline-trigger: --t view() / entry", "timeline-trigger: --t view() cover / entry 10% exit", "timeline-trigger: --t / entry", "timeline-trigger: --t, --u view()", "animation-trigger: --t", "animation-trigger: --t play", "animation-trigger: --t play-forwards play-backwards", "animation-trigger: none", "animation: spin 1s; animation-trigger: --t play", "font-style: oblique 0rad", "font-style: oblique 0turn", "font-style: oblique -0deg", "font-style: oblique 100grad", "font-style: oblique 91grad", "font-style: oblique 2rad", "font-style: oblique calc(91deg)", "font-style: oblique calc(10deg)", "font-style: oblique 90.5deg", "font-family: a  b", "font-family: a b", "font-family: -a", "font-family: _a", "font-family: a1 b", "font-family: a \\31", "font-family: é", "font-family: a\\-b", "font-family: \"a b\"", "font-family: \"ab\"", "font-family: \"serif\"", "font-family: \"1a\"", "font-family: a, \"b c\", --d", "font-family: -", "font-family: \"-\"", "font-family: \"\"", "font-family: a-b", "font-family: \"a-\"", "font-family: \"a\\\\b\"", "font-family: \"default\"", "font-family: \"inherit\"", "font-family: \"a\\a b\"", "animation-name: 'a b'", "animation-name: 'ab'", "animation-name: '1a'", "animation-name: 'a\\\\b'", "animation-name: '-a'", "animation-name: '--a'", "animation-name: ''", "animation-name: 'a\"b'", "animation-name: 'é'", "animation-name: '-1'", "animation-name: 'a.b'", "animation-name: 'none'", "animation-name: 'initial'", "animation-name: 'a\\a b'", "animation-name: '\\31 a'", "animation-name: 'a, b'", "animation: 'a b' 1s", "perspective-origin: left", "perspective-origin: top", "perspective-origin: 10px", "perspective-origin: center", "perspective-origin: left top", "perspective-origin: top left", "transform-origin: left", "transform-origin: top", "transform-origin: 10px", "transform-origin: top left 1px", "object-position: left", "object-position: top", "border-image-slice: fill 10", "border-image-slice: fill 10 20%", "border-image: url(a.png) fill 10", "offset-anchor: normal", "offset-position: left 10px top", "offset-anchor: left 10px top", "offset-position: left 10px top 5px", "offset-anchor: left 10px top 5px", "offset: left 10px top path('M 0 0')", "offset: path('M 0 0') / left 10px top", "background-position: left 10px top", "object-position: left 10px top", "transform-origin: left 10px top", "font-variant-alternates: swash(a) stylistic(b)", "font-variant-alternates: ornaments(b) swash(a)", "font-variant-alternates: historical-forms stylistic(a)", "font-variant-alternates: annotation(a) character-variant(b) styleset(c) historical-forms swash(d) ornaments(e) stylistic(f)", "font-variant-alternates: swash(a) swash(b)", "font-variant: swash(a) small-caps ornaments(b)", "font-variant: ornaments(b) swash(a)", "font-variant: historical-forms swash(a) small-caps", "background: red; background-position-y: 10px", "background: url(a.png); background-position-y: 10px", "background: url(a.png); background-position-y: 10px; background-size: 5px", "mask: url(a.png); -webkit-mask-position-x: 10px", "mask: url(a.png); -webkit-mask-position-y: 10px", "mask: url(a.png); -webkit-mask-position-x: 10px; mask-size: 5px", "mask: url(a.png); mask-position: left 10px top", "mask: none; mask-repeat: no-repeat", "mask: url(a.png), url(b.png); mask-composite: exclude", "mask: url(a.png), url(b.png); mask-clip: content-box", "mask: url(a.png), url(b.png); mask-origin: content-box", "background: url(a.png), url(b.png); background-origin: content-box", "background: url(a.png), url(b.png); background-attachment: fixed", "background: url(a.png), url(b.png); background-image: url(c.png)", "background: url(a.png), url(b.png); background-image: url(c.png), url(d.png), url(e.png)", "timeline-trigger: none; timeline-trigger-active-range-end: cover", "timeline-trigger: none; timeline-trigger-activation-range-end: cover", "timeline-trigger: --t view() entry / exit", "timeline-trigger: --t view() entry 10% exit / exit 10%", "timeline-trigger: --t / entry exit", "timeline-trigger: --t entry", "timeline-trigger: --t view() /", "timeline-trigger: / entry", "timeline-trigger: --t view() / auto", "timeline-trigger: --t view() normal / auto", "timeline-trigger: --t view(), --u scroll() / cover", "transform-origin: left 10px", "transform-origin: center top 1px", "transform-origin: 10px top", "transform-origin: top 10px", "transform-origin: center", "transform-origin: bottom right 0", "transform-origin: 1px 2px 3%", "perspective-origin: right 10px bottom 5px", "perspective-origin: 1px 2px 3px", "object-position: right 10px bottom 5px", "object-position: center center", "object-position: 10px", "offset-anchor: left top", "offset-anchor: top left", "offset-anchor: 10px", "offset-anchor: left", "offset-anchor: top", "offset-position: left", "offset-position: top", "offset: left top", "offset: top", "offset: path('M 0 0') / left", "offset: path('M 0 0') / top", "offset: left 10px top 5px path('M 0 0')", "transition-timing-function: steps(1.5)", "transition-timing-function: steps(calc(2))", "transition-timing-function: cubic-bezier(0.5, 2, 0.5, 0)", "transition-timing-function: cubic-bezier(1.5, 0, 0, 0)", "transition-timing-function: cubic-bezier(0, 0, 1.5, 0)", "transition-timing-function: steps(2, jump-both)", "transition-timing-function: steps(2, START)", "transition-timing-function: linear(0, 1 50% 25% )", "transition-timing-function: linear(0, 0.5 80%, 1 40%)", "transition-timing-function: linear(0, 0.5, 1 -10%)", "transition-timing-function: linear(0 50%, 1 20%)", "grid-template-columns: repeat(2, 1fr -1px)", "grid-template-columns: minmax(-1px, 1fr)", "grid-template-columns: fit-content(-1px)", "grid-template-columns: repeat(-1, 1fr)", "grid-template-columns: -1fr", "grid-template-columns: repeat(2, [a] 1fr)", "grid-template-columns: minmax(1px, -1fr)", "grid-template-columns: minmax(1fr, 1px)", "grid-template-columns: repeat(0, 1fr)", "grid-template-columns: minmax(10%, -5%)", "box-shadow: 0 0 1px -1px red", "box-shadow: 0 0 -0px red", "box-shadow: 0 0 calc(-1px) red", "animation-name: 'infinite'", "animation-name: 'ease'", "animation-name: 'default'", "animation-name: 'inherit'", "animation: 'infinite' 1s", "animation: 'ease' 1s", "animation: 'a b' 1s, 'none' 2s", "font-family: \"a\\\"b\"", "font-family: a\\\"b", "font-family: \"SERIF\"", "font-family: SERIF", "font-family: \"a\" b", "font-family: a  \"b\"", "font-family: Times New Roman", "font-family: \"Times New Roman\"", "font-family: a, -b, _c, \\-d", "font-family: system-ui", "font-family: \"system-ui\"", "font-family: -webkit-body", "font-family: math", "font-family: \"math\"", "font: 12px a b", "font: 12px \"a b\"", "font: 12px --a", "font: 12px -1a", "white-space-collapse: preserve-spaces", "white-space: preserve-spaces wrap", "white-space: break-spaces nowrap", "width: calc(1px + 1s)", "width: calc(1% + 1)", "width: calc(1px * 1%)", "width: calc(1% / 1%)", "width: calc(pi * 1px)", "width: calc(e * 1px)", "width: calc(1px + min(1px, 2px))", "width: calc((1px + 2%) * 2)", "width: calc((1in + 1px))", "width: calc(2 * (1px + 1em))", "width: calc(1px - (2px - 3px))", "width: calc(((1px)))", "width: calc(1px * (2 + 3))", "width: calc(1foo)", "width: calc(1px + 1deg)", "color: rgb(from red r g b)", "color: rgb(1 2 3 4)", "color: hsl(120deg 100% 50% 1)", "color: rgb(none 2 3)", "color: hsl(none 100% 50%)", "color: rgb(calc(1) 2 3)"]
for index, css in enumerate(PROBED):
    blocks[f"probed-{index}"] = [["css", css]]

# Round seven (coverage, P5-cov): serializations with mismatched lists / CSS-wide parts,
# custom properties, removal of an absent longhand, getPropertyPriority of a shorthand.
blocks.update({
    "cov-rule-inset-mismatch": [["css", "column-rule-inset: 1px; row-rule-inset: 2px"]],
    "cov-rule-inset-cap-mismatch": [["css", "column-rule-inset-cap: 1px; row-rule-inset-cap: 2px"]],
    "cov-border-block-mismatch": [["css", "border-block-start: 1px solid red; border-block-end: 2px solid blue"]],
    "cov-rule-mismatch": [["css", "column-rule: 1px solid red; row-rule: 2px solid blue"]],
    "cov-text-decoration-initial-line": [["css", "text-decoration-line: initial; text-decoration-thickness: auto; text-decoration-style: solid; text-decoration-color: currentcolor"]],
    "cov-columns-wrap": [["css", "columns: 2; column-wrap: wrap"]],
    "cov-columns-nowrap": [["css", "columns: 2; column-wrap: nowrap"]],
    "cov-white-space-initial": [["css", "white-space-collapse: initial; text-wrap-mode: nowrap"]],
    "cov-font-synthesis-initial": [["css", "font-synthesis-weight: initial; font-synthesis-style: auto; font-synthesis-small-caps: none"]],
    "cov-scroll-timeline-mismatch": [["css", "scroll-timeline: --a, --b; scroll-timeline-axis: x"]],
    "cov-view-timeline-mismatch": [["css", "view-timeline: --a, --b; view-timeline-axis: x"]],
    "cov-font-variant-none-caps": [["css", "font-variant: none; font-variant-caps: small-caps"]],
    "cov-grid-areas-column-flow": [["css", "grid: \"a\" / 1fr; grid-auto-flow: column"]],
    "cov-trigger-ranges-mismatch": [["css", "timeline-trigger: --t view(), --u view(); timeline-trigger-activation-range-end: normal"]],
    "cov-font-caps": [["css", "font: 12px a; font-variant-caps: all-small-caps"]],
    "cov-custom-property": [["set", "--a", " 1px "], ["get", "--a"], ["set", "--B", "x"], ["remove", "--a"]],
    "cov-remove-missing": [["set", "color", "red"], ["remove", "width"]],
    "cov-priority-shorthand": [["set", "margin", "1px", "important"], ["priority", "margin"], ["priority", "margin-top"], ["priority", "color"]],
    "cov-priority-mixed": [["set", "margin", "1px"], ["set", "margin-top", "2px", "important"], ["priority", "margin"], ["priority", "margin-left"]],
    "cov-priority-partial": [["set", "margin-top", "2px", "important"], ["priority", "margin"]],
})

scenarios = []
for name, vals in values.items():
    generic = ["inherit", "INITIAL", "unset", "revert", "revert-layer", "var(--x)", "var(--x) 1px", "", "bogus-value !"]
    for index, value in enumerate(vals):
        if value not in generic:
            scenarios.append({"id": f"{name}|{value}", "ops": [["set", name, value]], "assemble": index == 0})
    for value in generic:
        scenarios.append({"id": f"{name}|{value}", "ops": [["set", name, value]]})
    scenarios.append({"id": f"{name}|{vals[0]}|important", "ops": [["set", name, vals[0], "important"]]})
for name, ops in blocks.items():
    scenarios.append({"id": name, "ops": ops})
ids = [s["id"] for s in scenarios]
assert len(ids) == len(set(ids)), "duplicate scenario ids"
page = ('<!doctype html><html><head><script type="application/json" id="names">' + json.dumps(NAMES)
        + '</script><script type="application/json" id="scenarios">'
        + json.dumps(scenarios).replace("</", "<\\/") + '</script></head><body></body></html>')
with open(OUT, "w") as out:
    json.dump([{"id": "css-shorthand", "html": page}], out, indent=1)
    out.write("\n")
print(f"{len(scenarios)} scenarios over {len(values)} shorthands -> {OUT}")
