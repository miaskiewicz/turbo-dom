#!/usr/bin/env python3
"""Writes crates/turbo-dom/tests/fixtures/css-math-shape-cases.json (miaskiewicz/turbo-dom#338):
the [property, value] pairs of turbo-dom-browser-env's Chrome 156 control
css_tree_counting_shape_chrome156.json (tree-counting functions in math expressions, shape()
basic shapes) on the css-longhand-probe.js page, so rtdom::css_shorthand and the npm
runtime's css-shorthand.mjs spell them against the same control. Regenerate, then recapture
(from the repository root):
  python3 scripts/make-css-math-shape-cases.py
  node crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs \\
    --probe crates/turbo-dom/tests/fixtures/css-longhand-probe.js \\
    --cases crates/turbo-dom/tests/fixtures/css-math-shape-cases.json \\
    --out crates/turbo-dom/tests/fixtures/css_math_shape_chrome156.json --chrome <Chrome 156>
"""
import json
import os

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
FIXTURES = os.path.join(ROOT, "crates", "turbo-dom", "tests", "fixtures")
SOURCE = os.path.join(ROOT, "crates", "turbo-dom-browser-env", "src", "css_tree_counting_shape_chrome156.json")
result = json.load(open(SOURCE))["result"][0]["result"]
pairs = []
for group in ("tree", "shapes", "boxes"):
    for row in result[group]:
        if [row[0], row[1]] not in pairs:
            pairs.append([row[0], row[1]])
page = ('<!doctype html><html><head><script type="application/json" id="values">'
        + json.dumps(pairs).replace("</", "<\\/") + '</script></head><body></body></html>')
with open(os.path.join(FIXTURES, "css-math-shape-cases.json"), "w") as out:
    json.dump([{"id": "css-math-shape", "html": page}], out, indent=1)
    out.write("\n")
print(f"{len(pairs)} values -> css-math-shape-cases.json")
