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
