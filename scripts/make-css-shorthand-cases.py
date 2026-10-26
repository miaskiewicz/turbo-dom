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
for name, vals in edge.items():
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
}

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
