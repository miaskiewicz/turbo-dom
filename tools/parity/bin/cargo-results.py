#!/usr/bin/env python3
"""Turn raw `cargo test` output into sorted `binary<TAB>test<TAB>result` lines.

Usage: cargo-results.py RAW_LOG > results.tsv
Binary names have their hash suffix stripped so runs are comparable.
"""
import re
import sys

running = re.compile(r"^\s*Running (?:unittests )?(\S+)(?: \((.+)\))?")
doctests = re.compile(r"^\s*Doc-tests (\S+)")
case = re.compile(r"^test (.+?) \.\.\. (ok|FAILED|ignored(?:, .*)?)$")


def binary_name(src, target):
    path = target or src
    name = path.rsplit("/", 1)[-1]
    name = re.sub(r"-[0-9a-f]{16}(\.exe)?$", "", name)
    return f"{name}<{src}>" if target else name


def main():
    current = "?"
    rows = set()
    with open(sys.argv[1], errors="replace") as log:
        for line in log:
            line = line.rstrip("\n")
            m = running.match(line)
            if m:
                current = binary_name(m.group(1), m.group(2))
                continue
            m = doctests.match(line)
            if m:
                current = f"doctest:{m.group(1)}"
                continue
            m = case.match(line)
            if m:
                result = "ignored" if m.group(2).startswith("ignored") else m.group(2)
                rows.add((current, m.group(1), result))
    # Doctest names embed absolute registry/git checkout paths; make them host-independent.
    home = re.compile(r"/(?:Users|home)/[^/]+/\.cargo/(registry/src|git/checkouts)/[^/]+/")
    rows = {(b, home.sub(lambda m: "$CARGO/" + m.group(1) + "/", t), r) for b, t, r in rows}
    for row in sorted(rows):
        print("\t".join(row))


if __name__ == "__main__":
    main()
