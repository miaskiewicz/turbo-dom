#!/usr/bin/env python3
"""Render-parity harness driver.

Runs the pinned corpora through built napi addons, compares the outputs' sha256
manifests with a recorded checkpoint, and records new checkpoints.

  parity.py run OUT [--html2pdf A.node] [--html2pdf-svg B.node] [--surf C.node]
  parity.py compare OUT CHECKPOINT [--expect FILE ...]
  parity.py record OUT CHECKPOINT

`compare` fails on any output that differs from CHECKPOINT unless an expected-diff
file lists it. An expected-diff file is JSON:
  {"id": "turbo-html2pdf#29", "reason": "...", "control": "path/to/chrome-control",
   "changes": {"html2pdf": ["template-invoice-plain.pdf", ...], "surf": [...]},
   "added": {"html2pdf-extra": ["new-input.pdf"]}}
`changes` lists outputs the checkpoint has; `added` lists outputs of new corpus
inputs, which must be absent from the checkpoint. Entries listed but not produced
as described are reported as stale expectations (also a failure), so expectations
never outlive the change they describe.
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BIN = HERE / "bin"
MANIFESTS = HERE / "manifests"
CORPORA = {
    # corpus name -> (script, addon option)
    "html2pdf": ("corpus-html2pdf.mjs", "html2pdf"),
    "html2pdf-extra": ("corpus-html2pdf-extra.mjs", "html2pdf"),
    "html2pdf-svg": ("corpus-html2pdf.mjs", "html2pdf_svg"),
    "surf": ("corpus-surf.mjs", "surf"),
    "surf-extra": ("corpus-surf-extra.mjs", "surf"),
}


def run(args):
    out = Path(args.out)
    env = dict(os.environ, TZ="UTC", LANG="C")
    env.setdefault("PARITY_CACHE", str(HERE / ".cache"))
    ran = 0
    for name, (script, addon_opt) in CORPORA.items():
        addon = getattr(args, addon_opt)
        if not addon:
            continue
        subprocess.run(
            ["node", str(BIN / script), str(Path(addon).resolve()), str(out / name)],
            check=True,
            env=env,
        )
        ran += 1
    if not ran:
        sys.exit("run: pass at least one addon (--html2pdf / --html2pdf-svg / --surf)")


def load(path):
    return json.loads(Path(path).read_text())


def compare(args):
    out, checkpoint = Path(args.out), MANIFESTS / args.checkpoint
    if not checkpoint.is_dir():
        sys.exit(f"compare: unknown checkpoint {args.checkpoint}")
    expected, added = {}, {}
    for file in args.expect or []:
        spec = load(file)
        for corpus, keys in spec.get("changes", {}).items():
            for key in keys:
                expected.setdefault((corpus, key), []).append(spec["id"])
        # `added` declares outputs of new corpus inputs: they must be new, not changed.
        for corpus, keys in spec.get("added", {}).items():
            for key in keys:
                expected.setdefault((corpus, key), []).append(spec["id"])
                added[(corpus, key)] = True
    failures, compared = [], 0
    seen = set()
    for reference in sorted(checkpoint.glob("*.json")):
        corpus = reference.stem
        produced = out / corpus / "MANIFEST.json"
        if not produced.exists():
            continue
        compared += 1
        old, new = load(reference), load(produced)
        for key in sorted(set(old) | set(new)):
            if old.get(key) == new.get(key):
                continue
            seen.add((corpus, key))
            status = "changed" if key in old and key in new else ("added" if key in new else "removed")
            owners = expected.get((corpus, key))
            if added.get((corpus, key)) and status != "added":
                failures.append(f"NOT NEW {corpus}/{key} ({status}; declared added by {', '.join(owners)})")
            elif owners:
                print(f"expected  {corpus}/{key} ({status}; {', '.join(owners)})")
            else:
                failures.append(f"UNEXPECTED {corpus}/{key} ({status})")
    for (corpus, key), owners in sorted(expected.items()):
        if (corpus, key) not in seen and (out / corpus / "MANIFEST.json").exists():
            failures.append(f"STALE expectation {corpus}/{key} ({', '.join(owners)}) did not change")
    if not compared:
        sys.exit("compare: no produced corpus matches the checkpoint's corpora")
    for line in failures:
        print(line)
    print(f"{compared} corpora compared against {args.checkpoint}: {len(failures)} failure(s)")
    sys.exit(1 if failures else 0)


def record(args):
    out, checkpoint = Path(args.out), MANIFESTS / args.checkpoint
    checkpoint.mkdir(parents=True, exist_ok=True)
    for corpus in CORPORA:
        produced = out / corpus / "MANIFEST.json"
        if produced.exists():
            (checkpoint / f"{corpus}.json").write_text(produced.read_text())
            print(f"recorded {args.checkpoint}/{corpus}.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("run")
    p.add_argument("out")
    p.add_argument("--html2pdf")
    p.add_argument("--html2pdf-svg", dest="html2pdf_svg")
    p.add_argument("--surf")
    p.set_defaults(func=run)
    p = sub.add_parser("compare")
    p.add_argument("out")
    p.add_argument("checkpoint")
    p.add_argument("--expect", action="append")
    p.set_defaults(func=compare)
    p = sub.add_parser("record")
    p.add_argument("out")
    p.add_argument("checkpoint")
    p.set_defaults(func=record)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
