#!/usr/bin/env python3
"""Writes crates/turbo-dom/tests/fixtures/css-longhand-cases.json (miaskiewicz/turbo-dom#97): one
page whose <script id=values> lists [property, value] pairs css-longhand-probe.js sets on an
element's inline style (setProperty), reading back getPropertyValue and cssText: every longhand of
a Chrome 156 shorthand with the specified values Chrome spells in css_shorthand_chrome156.json,
each also written the way pages write it (upper case, a bare 0, url() without quotes, single
quotes, hex colors, extra spaces), plus common longhands outside shorthands. Regenerate, then
recapture the control (from the repository root):
  python3 scripts/make-css-longhand-cases.py
  node crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs \\
    --probe crates/turbo-dom/tests/fixtures/css-longhand-probe.js \\
    --cases crates/turbo-dom/tests/fixtures/css-longhand-cases.json \\
    --out crates/turbo-dom/tests/fixtures/css_longhand_chrome156.json --chrome <Chrome 156>
"""
import json
import os
import re

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
FIXTURES = os.path.join(ROOT, "crates", "turbo-dom", "tests", "fixtures")
control = json.load(open(os.path.join(FIXTURES, "css_shorthand_chrome156.json")))["result"][0]["result"]
WIDE = {"initial", "inherit", "unset", "revert", "revert-layer", ""}
seen = {}
for row in control["results"]:
    for item in row["items"]:
        if item[1] not in WIDE and "initial" not in item[1].split(", "):
            seen.setdefault(item[0], [])
            if item[1] not in seen[item[0]]:
                seen[item[0]].append(item[1])


def hexed(value):
    def one(match):
        r, g, b = (int(v) for v in match.groups())
        return "#%02x%02x%02x" % (r, g, b)
    return re.sub(r"rgb\((\d+), (\d+), (\d+)\)", one, value)


def variants(value):
    out = [value, value.upper(), re.sub(r"\b0px\b", "0", value), value.replace('url("', "url(").replace('")', ")"),
           value.replace('"', "'"), hexed(value), value.replace(" ", "  "), " " + value + " "]
    unique = []
    for v in out:
        if v not in unique:
            unique.append(v)
    return unique

pairs = []
for name in sorted(seen):
    for value in seen[name][:3]:
        for variant in variants(value):
            pairs.append([name, variant])
EXTRA = {
    "width": ["0", "10PX", "50%", "auto", "calc(100% - 10px)", "min-content", "fit-content", "-1px", "10"],
    "height": ["0", "1E1px", "100vh", "AUTO"],
    "min-width": ["0", "auto"], "max-width": ["none", "0", "100%"], "max-height": ["NONE"],
    "color": ["RED", "#FFF", "rgb(1,2,3)", "Transparent", "currentColor", "hsl(0, 100%, 50%)", "bogus"],
    "display": ["BLOCK", "inline flex", "Flex", "none", "grid", "contents", "bogus"],
    "position": ["ABSOLUTE", "sticky", "relative"],
    "visibility": ["HIDDEN", "collapse"], "float": ["LEFT", "none"], "box-sizing": ["Border-Box"],
    "text-align": ["CENTER", "-webkit-center", "start"], "cursor": ["POINTER", "url(a.png), auto"],
    "z-index": ["10", "AUTO", "-1", "1.5"], "opacity": ["0.5", "50%", "1", ".5", "2"],
    "letter-spacing": ["0", "NORMAL", "1PX"], "word-spacing": ["0"], "text-indent": ["0", "5%"],
    "line-height": ["0", "1.5", "NORMAL", "20PX", "150%"], "font-size": ["12PX", "Small", "0", "1.2em", "larger"],
    "font-weight": ["BOLD", "700", "normal", "550", "1001"], "font-family": ["'Times New Roman', serif", "Arial", "'Arial'", "\"a\", \"serif\"", "a  b"],
    "transform": ["none", "translate(10px, 0)", "rotate(45DEG)", "scale(2)"],
    "box-shadow": ["none", "0 0 1px red", "1PX 1px #000"],
    "background-image": ["url(a.png)", "none", "linear-gradient(red,blue)"],
    "transition-duration": ["1S", "200MS", "0s, 1s"], "animation-name": ["Spin", "'spin'", "none"],
    "overflow-wrap": ["ANYWHERE"], "word-wrap": ["break-word"], "white-space-collapse": ["PRESERVE"],
    "column-rule-color": ["red", "#00ff00"], "row-rule-color": ["red"], "position-try-fallbacks": ["--a", "--a, --b", "flip-block"],
    "interest-delay-start": ["inherit", "1s", "NORMAL"], "interest-delay-end": ["initial", "200ms"],
    "top": ["0", "1PX", "AUTO", "calc(1px + 2px)"], "margin-top": ["0", "1PX", "Auto", "-0", "calc(1em + 1ch)", "calc(1ch + 1em)", "calc(1px + 1em)", "calc(1vw + 1px + 1rem)", "calc(1in + 1cm)", "calc(1px + 1q)", "calc(2% + 1px + 1em)", "calc(1in)"],
    "animation-duration": ["auto", "1S", "-1s"], "text-align": ["-WEBKIT-CENTER", "bogus"], "font-variant-alternates": ["swash(a)", "bogus"],
    "min-height": ["fit-content(10px)", "anchor-size(width)"],
    "border-top-color": ["#abc", "RED"], "outline-color": ["Red"], "flex-basis": ["0", "10PX", "AUTO", "Content"],
    "flex-grow": ["1.50", "+2", "0"],
}
# Round two (mutation run): sign and range checks, list forms, escapes.
MUTATION = {
    "flex-grow": ["-1"], "flex-shrink": ["-1", "0.5"], "column-count": ["0", "1", "-1", "1.5", "AUTO"],
    "column-width": ["-1px", "0", "10%"], "column-height": ["-1px"], "font-size": ["-1px", "-10%", "10%"],
    "line-height": ["-1", "-1px", "-10%", "0.5"], "font-weight": ["0", "1", "1000", "0.5", "999.5", "1000.5", "BOLDER", "-1"],
    "font-stretch": ["-10%", "0%", "50%", "87.5%", "CONDENSED"], "font-style": ["oblique -90deg", "oblique 91deg", "oblique 0deg", "oblique 14deg", "oblique 10deg", "oblique", "OBLIQUE 1turn", "italic 10deg"],
    "list-style-image": ["url(a.png)", "none", "linear-gradient(red, blue)", "bogus"],
    "background-image": ["image-set(url(a.png) 1x)", "url(a.png), none", "url(a.png),", "bogus"], "mask-image": ["url(a.png)", "none, url(b.png)"],
    "transition-timing-function": ["ease, linear", "steps(2)", "bogus", "linear(0, 1)"], "animation-timing-function": ["ease-in-out, steps(1, start)"],
    "grid-template-columns": ["-1px", "10% 1fr", "-10%", "repeat(2, 1fr)", "minmax(10px, 1fr) auto", "[a] 1fr [b]", "bogus", "subgrid", "none"],
    "grid-template-rows": ["1fr -1fr", "fit-content(10px)", "auto 0"],
    "z-index": ["1e1", "+1"], "offset-distance": ["-10px", "-10%", "10px"], "offset-anchor": ["auto", "normal", "left top", "10px 20px", "bogus"],
    "offset-position": ["normal", "left 10px top", "auto", "center"], "animation-range-start": ["-10%", "cover -10%", "normal", "contain 10px", "bogus"],
    "animation-range-end": ["-10%", "contain 110%", "normal", "exit"], "transition-duration": ["-1s", "1s, -1s"], "transition-delay": ["-1s", "-1s, 1s"],
    "animation-delay": ["-1s"], "border-image-width": ["-1", "-1px", "1 2px auto 10%"], "border-image-outset": ["-1", "-1px", "1 2px"],
    "border-image-slice": ["-1", "10% fill", "fill 10", "1 2 3 4 5"], "text-decoration-thickness": ["-1px", "-10%", "auto", "from-font"],
    "text-underline-offset": ["-1px", "auto"], "outline-width": ["-1px", "thin"], "outline-offset": ["-1px"], "scroll-padding-top": ["-1px", "auto"],
    "scroll-margin-top": ["-1px", "auto"], "padding-top": ["-1px", "-5%"], "border-top-width": ["-1px", "THICK"], "row-gap": ["-1px", "normal"],
    "background-size": ["-1px", "10px -1px", "cover", "auto 10%", "10px, contain"], "background-position-x": ["-10px", "left -10px", "right 10px", "center", "left, right 10%"],
    "background-position-y": ["bottom -10%", "center", "top 10px", "10px, bottom"], "background-repeat": ["repeat no-repeat", "repeat-x, space"],
    "opacity": ["-1", "1e-6", "0.000001", "1e6", "-10%"], "flex-basis": ["-1px", "-10%", "max-content"], "max-width": ["-1px"], "min-height": ["-1px"],
    "grid-row-start": ["0", "span 0", "-1", "span a 2", "a 2"], "column-gap": ["-1px"], "letter-spacing": ["-1px"], "text-indent": ["-1px"],
    "font-family": ["a\\ b", "\"a\\\\b\"", "'a\"b'", "a,", "inherit a", "\\61", "--a", "-1a"], "content": ["\"a\\\\b\"", "\"a\\a b\"", "'a'", "\"a\\\"b\"", "\"a"],
    "quotes": ["'a' 'b'", "\"a\\\\\" \"b\""], "box-shadow": ["inset 0 0 1px red", "0 0 1px red inset", "inset inset 0 0", "0 0 -1px red", "0 0 1px -1px red",
    "0 0 1px 1px 1px red", "0 0", "0", "red", "0 0 1px red, inset 1px 1px blue", "red 0 0 red"],
    "transition-property": ["opacity, all", "none", "none, opacity", "--a", "initial"], "animation-name": ["a, b", "none, a", "'a b'", "\"inherit\"", "'none'"],
    "transform-origin": ["left top", "10px 20px 30px"], "perspective-origin": ["left"], "object-position": ["right 10px bottom"],
    "width": ["calc(1mm + 1px)", "calc(1pt + 1px)", "calc(1pc + 1px)", "calc(2px * 3)", "calc(1px / 2px)", "calc(1px * 2px)", "min(1px, 2%)"],
    "color": ["hsl(60, 100%, 50%)", "hsl(180 50% 25%)", "hsl(270deg 100% 50% / 0.5)", "rgb(1 2 3 / 0.5)", "#\\31 23", "#abcd", "#aabbccdd", "rgb(300, -1, 2)"],
}
for name, values in MUTATION.items():
    for value in values:
        if [name, value] not in pairs:
            pairs.append([name, value])
for name, values in EXTRA.items():
    for value in values:
        if [name, value] not in pairs:
            pairs.append([name, value])
page = ('<!doctype html><html><head><script type="application/json" id="values">'
        + json.dumps(pairs).replace("</", "<\\/") + '</script></head><body></body></html>')
with open(os.path.join(FIXTURES, "css-longhand-cases.json"), "w") as out:
    json.dump([{"id": "css-longhand", "html": page}], out, indent=1)
    out.write("\n")
print(f"{len(pairs)} longhand values -> css-longhand-cases.json")
