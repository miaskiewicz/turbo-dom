#!/usr/bin/env python3
"""Check `selector-dual` JSONL logs (turbo-dom consolidation S3a/S3b, docs/selector-dual-gate.md).

usage: selector-dual-check.py --mode legacy|new --manifest MANIFEST.json [--classify]
                              [--allow-vacuous SUITE]... [--report OUT.json]
                              SUITE=LOGDIR [SUITE=LOGDIR ...]

Per suite (every `*.jsonl` under LOGDIR, one file per process):
  * headers: every header's mode is --mode, its frozen_sha is the expected one and its
    build_id is one of the manifest's `build_ids` (no stale dual artifact can pass);
  * vacuity: at least one header AND at least one compared call (max `count` record);
  * --classify (Gate B): every `divergence` record matches the classifier of ONE bug-list
    entry of miaskiewicz/turbo-dom#18 (S3a list), testing BOTH the selector feature and the
    element-side condition on every element the answers disagree on. Unclassified -> fail.
Exit status 1 on any failure.
"""
import argparse
import glob
import json
import os
import re
import sys

FROZEN_SHA = "4d5a63df36f3f94fcfb22b184039b25afd485640"

# ------------------------------------------------------------------ selector reading
HEX = set("0123456789abcdefABCDEF")
SPECIAL = set(" \t\n\r\f.#:[]()>+~,=|*\"'^$!/")


def runs(selector):
    """CSS identifier-ish runs of `selector`: (decoded text, had_escape, had_non_ascii).

    A run is a maximal sequence of name code points and escapes; quoted strings are one
    run each (their content decoded). Separators are the selector punctuation."""
    out = []
    i, n = 0, len(selector)
    cur, esc, non_ascii = [], False, False

    def flush():
        nonlocal cur, esc, non_ascii
        if cur:
            out.append(("".join(cur), esc, non_ascii))
        cur, esc, non_ascii = [], False, False

    def escape(i):
        """decode the escape at selector[i] == '\\' -> (text, next index)"""
        j = i + 1
        k = j
        while k < n and k < j + 6 and selector[k] in HEX:
            k += 1
        if k > j:
            cp = int(selector[j:k], 16)
            if k < n and selector[k] in " \t\n\f":
                k += 1
            elif k + 1 < n and selector[k : k + 2] == "\r\n":
                k += 2
            elif k < n and selector[k] == "\r":
                k += 1
            text = chr(cp) if 0 < cp <= 0x10FFFF and not 0xD800 <= cp <= 0xDFFF else "�"
            return text, k
        if j < n:
            return selector[j], j + 1
        return "�", j

    while i < n:
        c = selector[i]
        if c == "\\":
            text, i = escape(i)
            cur.append(text)
            esc = True
            continue
        if c in "\"'":
            flush()
            q, i = c, i + 1
            buf, e = [], False
            while i < n and selector[i] != q:
                if selector[i] == "\\":
                    t, i = escape(i)
                    buf.append(t)
                    e = True
                else:
                    buf.append(selector[i])
                    i += 1
            i += 1
            text = "".join(buf)
            out.append((text, e, any(ord(ch) > 127 for ch in text)))
            continue
        if c in SPECIAL:
            flush()
        else:
            cur.append(c)
            if ord(c) > 127:
                non_ascii = True
        i += 1
    flush()
    return out


def without_brackets(selector):
    return re.sub(r"\[[^\]]*\]?", "[]", selector)


NS_TYPE = re.compile(r"(?:^|[\s>+~(,])(\*?)\|(\*|[^\s>+~(),.#:\[\]|]+)")
NTH_OF = re.compile(r":nth-(?:last-)?child\([^)]*\sof(?:[\s.#\[:*|]|$)")


def tokens(element):
    return set(element.get("facts", {}).get("tokens", []))


def every(record, test):
    els = record.get("elements") or []
    return bool(els) and all(test(e) for e in els)


def validity(record):
    """`_checked` validity changes: (legacy error char, new is ok)."""
    legacy, new = record.get("legacy"), record.get("new")
    if not (isinstance(legacy, dict) and isinstance(new, dict)):
        return None
    if "err" in legacy and "ok" in new:
        m = re.match(r"unexpected '(.)' in selector", legacy["err"])
        return m.group(1) if m else ""
    return None


def c_escapes(sel, rec):
    if "\\" not in sel:
        return False
    if validity(rec) == "\\":
        return True
    escaped = {t for t, e, _ in runs(sel) if e}
    return bool(escaped) and every(rec, lambda el: bool(escaped & tokens(el)))


def c_non_ascii(sel, rec):
    if not any(ord(c) > 127 for c in sel):
        return False
    v = validity(rec)
    if v is not None and v != "" and ord(v) > 127:
        return True
    wide = {t for t, _, a in runs(sel) if a}
    return bool(wide) and every(rec, lambda el: bool(wide & tokens(el)))


def c_namespace(sel, rec):
    found = NS_TYPE.findall(without_brackets(sel))
    if not found:
        return False
    if validity(rec) == "|":
        return True
    names = {name.lower() for _, name in found}
    return every(rec, lambda el: "*" in names or bool(names & {t.lower() for t in tokens(el)}))


def c_empty(sel, rec):
    return ":empty" in sel and every(rec, lambda el: el["facts"]["empty_ignorable"])


def c_read_write(sel, rec):
    return (":read-only" in sel or ":read-write" in sel) and every(
        rec, lambda el: el["facts"]["rw_differs"]
    )


def c_nth_of(sel, rec):
    return bool(NTH_OF.search(sel)) and every(rec, lambda el: el["facts"]["has_parent"])


def c_link(sel, rec):
    return (":link" in sel or ":any-link" in sel) and every(rec, lambda el: el["facts"]["link"])


# One classifier per S3a bug-list entry (miaskiewicz/turbo-dom#18).
CLASSIFIERS = [
    ("escapes", c_escapes),
    ("non-ascii", c_non_ascii),
    ("namespace *|p", c_namespace),
    (":empty", c_empty),
    (":read-only/:read-write", c_read_write),
    (":nth-child(An+B of S)", c_nth_of),
    (":link/:any-link", c_link),
]


def classify(record):
    sel = record.get("selector", "")
    for name, test in CLASSIFIERS:
        try:
            if test(sel, record):
                return name
        except (KeyError, TypeError, ValueError):
            continue
    return None


# ------------------------------------------------------------------ main
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mode", required=True, choices=["legacy", "new"])
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--classify", action="store_true")
    ap.add_argument("--report")
    ap.add_argument("--allow-vacuous", action="append", default=[],
                    help="suite whose tests make no selector call (results-only; documented)")
    ap.add_argument("suites", nargs="+")
    args = ap.parse_args()
    manifest = json.load(open(args.manifest))
    build_ids = set(manifest.get("build_ids", []))
    failed = False
    report = {"mode": args.mode, "suites": {}}
    for spec in args.suites:
        suite, _, logdir = spec.partition("=")
        files = sorted(glob.glob(os.path.join(logdir, "**", "*.jsonl"), recursive=True))
        headers, compared, problems = 0, {}, []
        classes, unclassified, divergences = {}, [], 0
        for path in files:
            for line in open(path, errors="replace"):
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    problems.append(f"{path}: unparsable line")
                    continue
                kind = rec.get("type")
                if kind == "header":
                    headers += 1
                    if rec.get("mode") != args.mode:
                        problems.append(f"{path}: header mode {rec.get('mode')}")
                    if rec.get("frozen_sha") != FROZEN_SHA:
                        problems.append(f"{path}: frozen_sha {rec.get('frozen_sha')}")
                    if rec.get("build_id") not in build_ids:
                        problems.append(f"{path}: build_id {rec.get('build_id')} not in manifest")
                elif kind == "count":
                    compared[path] = max(compared.get(path, 0), rec.get("compared", 0))
                elif kind == "divergence":
                    divergences += 1
                    if args.classify:
                        name = classify(rec)
                        if name:
                            classes[name] = classes.get(name, 0) + 1
                        else:
                            unclassified.append(rec)
        total = sum(compared.values())
        vacuous_ok = suite in args.allow_vacuous
        if headers == 0 and not vacuous_ok:
            problems.append("vacuous: no header records")
        if total == 0 and not vacuous_ok:
            problems.append("vacuous: no compared calls")
        if unclassified:
            problems.append(f"{len(unclassified)} unclassified divergences")
        ok = not problems
        failed |= not ok
        report["suites"][suite] = {
            "files": len(files),
            "headers": headers,
            "compared_at_least": total,
            "divergences": divergences,
            "classes": classes,
            "unclassified": unclassified[:50],
            "problems": problems[:50],
            "ok": ok,
            "vacuity_waived": vacuous_ok,
        }
        status = ("ok" if headers else "ok, results only") if ok else "FAIL"
        cls = ", ".join(f"{k}={v}" for k, v in sorted(classes.items()))
        print(
            f"[{status}] {suite}: {len(files)} logs, {headers} headers, >= {total} compared, "
            f"{divergences} divergences" + (f" ({cls})" if cls else "")
        )
        for p in problems[:20]:
            print(f"    {p}")
        for rec in unclassified[:10]:
            print(f"    UNCLASSIFIED {rec.get('api')} {rec.get('selector')!r} legacy={rec.get('legacy')} new={rec.get('new')}")
    if args.report:
        json.dump(report, open(args.report, "w"), indent=1)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
