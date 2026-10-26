#!/usr/bin/env python3
"""Run a cargo test configuration that has a known, exact set of pre-existing failures.

  check-baseline-red.py LIST [--cargo CARGO] -- <cargo test arguments>

LIST holds one `binary<TAB>test` line per baseline-red test (binary as printed by
tools/parity/bin/cargo-results.py, e.g. `layout_flex<tests/layout_flex.rs>`;
`#` comments allowed). Two runs:

1. `cargo test ARGS --no-fail-fast -- --exact --skip <each test>` must pass: every
   test outside the list is green.
2. `cargo test ARGS --no-fail-fast -- --exact <each test>` runs only the listed tests;
   the set of (binary, test) that FAILED must equal LIST exactly. A listed test that
   now passes, is missing or is ignored fails the check (the list is stale: remove it),
   so the list can only shrink as the underlying bug is fixed.
"""
import argparse
import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location("cargo_results", REPO / "tools/parity/bin/cargo-results.py")
CARGO_RESULTS = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(CARGO_RESULTS)


def load_list(path):
    entries = set()
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        binary, sep, test = line.partition("\t")
        if not sep or not binary or not test or "\t" in test:
            sys.exit(f"{path}:{number}: expected 'binary<TAB>test'")
        entries.add((binary, test))
    if not entries:
        sys.exit(f"{path}: empty baseline-red list")
    return entries


def parse(output):
    """(binary, test) -> result for every test line of a cargo test log."""
    results, current = {}, "?"
    for line in output.splitlines():
        m = CARGO_RESULTS.running.match(line)
        if m:
            current = CARGO_RESULTS.binary_name(m.group(1), m.group(2))
            continue
        m = CARGO_RESULTS.doctests.match(line)
        if m:
            current = f"doctest:{m.group(1)}"
            continue
        m = CARGO_RESULTS.case.match(line)
        if m:
            results[(current, m.group(1))] = "ignored" if m.group(2).startswith("ignored") else m.group(2)
    return results


def cargo(command, label):
    print(f"$ {' '.join(command[:12])}{' ...' if len(command) > 12 else ''}", flush=True)
    run = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    sys.stdout.write(run.stdout[-20000:] if run.returncode else "")
    return run.returncode, run.stdout


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("list")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("cargo_args", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    cargo_args = args.cargo_args[1:] if args.cargo_args[:1] == ["--"] else args.cargo_args
    red = load_list(args.list)
    names = sorted({test for _, test in red})
    base = [args.cargo, "test", *cargo_args, "--no-fail-fast", "--", "--exact"]

    code, out = cargo(base + [arg for name in names for arg in ("--skip", name)], "green")
    green = parse(out)
    failed = sorted(key for key, result in green.items() if result == "FAILED")
    if code != 0 or failed:
        for binary, test in failed:
            print(f"NEW FAILURE {binary} {test}")
        sys.exit(f"check-baseline-red: tests outside {args.list} failed (cargo exit {code})")
    print(f"check-baseline-red: {sum(r == 'ok' for r in green.values())} tests outside the list pass")

    _, out = cargo(base + names, "red")
    ran = parse(out)
    status = 0
    for key in sorted(red):
        result = ran.get(key)
        if result != "FAILED":
            print(f"STALE {key[0]} {key[1]}: {result or 'did not run'} (remove it from {args.list})")
            status = 1
    for key in sorted(set(ran) - red):
        if ran[key] == "FAILED":
            print(f"UNLISTED FAILURE {key[0]} {key[1]}")
            status = 1
    if status:
        sys.exit(f"check-baseline-red: {args.list} does not match the failing set")
    print(f"check-baseline-red: exactly the {len(red)} listed tests fail ({args.list})")


if __name__ == "__main__":
    main()
