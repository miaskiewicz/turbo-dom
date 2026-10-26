#!/usr/bin/env python3
"""Check `selector-dual` JSONL logs (turbo-dom consolidation S3a/S3b, docs/selector-dual-gate.md).

usage: selector-dual-check.py --mode legacy|new --manifest MANIFEST.json [--classify]
                              [--allow-vacuous SUITE]... [--report OUT.json]
                              SUITE=LOGDIR [SUITE=LOGDIR ...]

Per suite (every `*.jsonl` under LOGDIR, one file per process):
  * headers: every header's mode is --mode, its frozen_sha is the expected one and its
    build_id is one of the manifest's `build_ids` (no stale dual artifact can pass);
  * vacuity: at least one header AND at least one compared call (max `count` record);
  * --classify (Gate B): every `divergence` record matches the classifier of ONE class,
    testing BOTH the selector feature and the element-side condition on every element the
    answers disagree on. Unclassified -> fail.
      - live-Tree records (matches / querySelector*): since S3b the frozen engine is the S3a
        engine, which the live Tree must still agree with exactly, so NO live record is
        classifiable (the S3a classifiers below stay for their unit tests and history);
      - cascade records (`"engine": "cascade"`, turbo-dom-layout): one S3b quirk class of
        the old html2pdf engine each (CASCADE_CLASSIFIERS), backed by a Chrome control.
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


# ------------------------------------------------------------------ S3b cascade classes
# The old html2pdf engine's pseudo-class names (case-sensitive: it never folded case) and
# the pseudo-elements it knew. Anything else it dropped from the compound, so the rest
# matched as if the pseudo were absent.
OLD_PSEUDO_CLASSES = {
    "first-child", "last-child", "only-child", "first-of-type", "last-of-type", "only-of-type",
    "nth-child", "nth-of-type", "root", "empty", "checked", "enabled", "disabled", "not", "is",
    "where", "matches", "-webkit-any", "-moz-any", "link", "any-link", "hover", "focus",
    "focus-within", "focus-visible", "active", "target", "visited", "autofocus", "default",
    "placeholder-shown",
}
OLD_PSEUDO_ELEMENTS = {
    "before", "after", "first-line", "first-letter", "marker", "selection", "placeholder",
    "backdrop", "cue", "file-selector-button", "target-text", "grammar-error", "spelling-error",
    "highlight",
}
# Pseudo-class names Chrome rejects although the old engine accepted them.
CHROME_INVALID = {"matches", "-moz-any", "autofocus", "selected"}
PSEUDO = re.compile(r"(::?)(-?[A-Za-z][-A-Za-z0-9]*)")
LEGACY_ELEMENTS = {"before", "after", "first-line", "first-letter"}


def pseudos(sel):
    """(colons, name) of every pseudo outside attribute brackets, strings and escapes."""
    unescaped = re.sub(r"\\[0-9a-fA-F]{1,6}\s?|\\.", "x", sel)
    return PSEUDO.findall(re.sub(r"\[[^\]]*\]?|\"[^\"]*\"|'[^']*'", "[]", unescaped))


def has_pseudo_element(sel):
    return any(c == "::" or n.lower() in LEGACY_ELEMENTS for c, n in pseudos(sel))


def cascade_selector(rec):
    return rec.get("selector") or rec.get("prelude") or ""


def element_tokens(el):
    out = {el.get("tag") or ""} | set(el.get("facts", {}).get("ancestors") or [])
    for name, value in (el.get("attrs") or []) + (el.get("local_attrs") or []):
        out |= {name, value, *value.split()}
    return {t for t in out if t}


def match_records(rec):
    return rec.get("api") in ("cascade_match", "cascade_rule_match")


def k_escapes(sel, rec):
    r"""Escapes: the old engine read `\` as a separator (`.md\:flex` -> `.md` + a dropped
    pseudo, `\70` -> `*`). A new match must contain a decoded escaped identifier the element
    (or an ancestor tag) carries. Control: S3a case "escapes"."""
    if "\\" not in sel:
        return False
    if not match_records(rec) or rec.get("new") is not True:
        return True
    escaped = {t for t, e, _ in runs(sel) if e}
    return bool(escaped & element_tokens(rec["element"]))


UNDECLARED_PREFIX = re.compile(r"(?:^|[\s>+~(,])[A-Za-z_-][\w-]*\|")


def k_namespace(sel, rec):
    """`*|`/`|` type prefixes: unknown to the old engine. A new match must name the
    element's type (or `*`); an undeclared prefix (`svg|rect`) invalidates the rule (no
    `@namespace` in these sheets). Control: S3a case "namespace"."""
    found = NS_TYPE.findall(without_brackets(sel))
    if not found:
        return rec.get("new") is False and bool(UNDECLARED_PREFIX.search(without_brackets(sel)))
    if not match_records(rec) or rec.get("new") is not True:
        return True
    names = {name.lower() for _, name in found}
    return "*" in names or (rec["element"].get("tag") or "").lower() in names


ATTR_NAME = re.compile(r"\[\s*([^\s~|^$*=\]]+)")


def k_attribute_namespace(sel, rec):
    """`[name]` matched a NAMESPACED attribute (`xlink:href`) by its local name in the old
    engine; Chrome matches no-namespace attributes only. Control: cascade case
    "attribute-namespace"."""
    if not match_records(rec) or rec.get("legacy") is not True or rec.get("new") is not False:
        return False
    el = rec["element"]
    qualified = {n for n, _ in el.get("attrs") or []}
    local = {n for n, _ in el.get("local_attrs") or []}
    namespaced = {n for n in local if n not in qualified}
    return bool({n.lower() for n in ATTR_NAME.findall(sel)} & namespaced)


def k_read_write(sel, rec):
    """`:read-only`/`:read-write`: dropped by the old engine. Control: S3a case
    "read-only-read-write"."""
    return ":read-only" in sel or ":read-write" in sel


def k_required_optional(sel, rec):
    """`:required`/`:optional`: dropped by the old engine. Control: S3a case "guards",
    cascade case "unsupported-pseudo-class"."""
    return ":required" in sel or ":optional" in sel


def k_nth_last(sel, rec):
    """`:nth-last-child()`/`:nth-last-of-type()`: dropped by the old engine. Controls: S3a
    case "nth-of-s" guards, cascade case "unsupported-pseudo-class"."""
    return ":nth-last-child(" in sel or ":nth-last-of-type(" in sel


def k_nth_of(sel, rec):
    """`:nth-child(An+B of S)`: the old engine parsed `An+B of S` as garbage. Control: S3a
    case "nth-of-s"."""
    return bool(NTH_OF.search(sel))


def k_has(sel, rec):
    """`:has()`: dropped by the old engine. Controls: S3a case "guards", cascade cases
    "unsupported-pseudo-class", "specificity-is-not-has"."""
    return ":has(" in sel


def k_pseudo_element(sel, rec):
    """A pseudo-element selector never matches an element. The old engine matched those it
    did not know (`::-webkit-scrollbar`, dropped) and counted them as pseudo-classes in
    specificity (inert: no element match). Control: cascade case
    "specificity-pseudo-element"."""
    if not has_pseudo_element(sel):
        return False
    if match_records(rec):
        return rec.get("legacy") is True and rec.get("new") is False
    return True


def k_unknown_pseudo(sel, rec):
    """A pseudo-class the old engine did not know (or wrote in another case) was dropped, so
    the compound matched as if it were absent; Chrome invalidates the rule when the name is
    unknown and otherwise evaluates it (`:lang()`, `:defined`, `:scope`, `:FIRST-CHILD`).
    Names Chrome rejects although the old engine knew them invalidate the rule too.
    Controls: cascade cases "unknown-pseudo-class", "cascade-only-pseudos"."""
    names = [n for c, n in pseudos(sel) if c == ":" and n.lower() not in LEGACY_ELEMENTS]
    return any(n not in OLD_PSEUDO_CLASSES or n.lower() in CHROME_INVALID for n in names)


SPEC_FEATURES = re.compile(r":(?:is|not|where|has|-webkit-any|nth-(?:last-)?child)\(", re.I)


def k_specificity(sel, rec):
    """Specificity: the old engine counted every functional pseudo as one pseudo-class;
    Chrome gives `:where()` 0 and `:is()`/`:not()`/`:has()` their most specific argument,
    `of S` adds S. Control: cascade cases "specificity-where", "specificity-is-not-has"."""
    return rec.get("api") == "cascade_specificity" and bool(SPEC_FEATURES.search(sel))


# One classifier per S3b quirk class, most specific first. A cascade_validity /
# cascade_list / cascade_rule_match record (a whole prelude) is explained by any class
# whose selector feature the prelude contains.
CASCADE_CLASSIFIERS = [
    ("escapes", k_escapes),
    ("namespace *|p", k_namespace),
    ("attribute-namespace", k_attribute_namespace),
    (":read-only/:read-write", k_read_write),
    (":required/:optional", k_required_optional),
    (":nth-last-*", k_nth_last),
    (":nth-child(An+B of S)", k_nth_of),
    (":has()", k_has),
    ("pseudo-element", k_pseudo_element),
    ("specificity", k_specificity),
    ("unknown-pseudo", k_unknown_pseudo),
]


def classify_cascade(record):
    if record.get("engine") != "cascade":
        return None
    sel = cascade_selector(record)
    for name, test in CASCADE_CLASSIFIERS:
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
                        name = classify_cascade(rec)
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
            print(f"    UNCLASSIFIED {rec.get('api')} {cascade_selector(rec)!r} legacy={rec.get('legacy')} new={rec.get('new')}")
    if args.report:
        json.dump(report, open(args.report, "w"), indent=1)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
