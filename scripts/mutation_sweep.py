#!/usr/bin/env python3
"""Bounded #487 mutation jobs; raw outcomes are evidence, never an allowlist."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "scripts/mutation-sweep-plan.json"
TARGETS = {
    "event_handlers": ("turbo-dom-browser-env", "host-internals,test-support"),
    "worker_scope": ("turbo-dom-browser-env", "host-internals,test-support"),
    "trusted_types": ("turbo-dom-browser-env", "host-internals,test-support"),
    "paint_png": ("turbo-dom-raster", ""),
    "paint_svg": ("turbo-dom-raster", ""),
}
TOOL_VERSION = "27.1.0"


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def capture(argv):
    return subprocess.check_output(argv, cwd=ROOT, text=True).strip()


def target_path(target):
    package, _ = TARGETS[target]
    return f"crates/{package}/src/{target}.rs"


def matrix(mode):
    plan = json.loads(PLAN.read_text())
    mode = mode or plan["mode"]
    if mode not in {"pilot", "full"}:
        raise ValueError("mode must be pilot or full")
    if set(plan["full_shards"]) != set(TARGETS):
        raise ValueError("all five targets must be present")
    jobs = []
    for target in TARGETS:
        count = 1 if mode == "pilot" else plan["full_shards"][target]
        if not isinstance(count, int) or not 1 <= count <= 12:
            raise ValueError("each target must have between 1 and 12 shards")
        for shard in range(count):
            jobs.append({"id": f"{target}-{shard}", "target": target,
                         "shard": shard, "shards": count, "mode": mode,
                         "browser": TARGETS[target][0] == "turbo-dom-browser-env"})
    if len(jobs) > 32:
        raise ValueError("matrix exceeds the reviewed 32-job ceiling")
    return {"include": jobs}


def rust_regex_literal(text):
    # Match regex::escape without Python's unnecessary escaped whitespace.
    return "".join("\\" + c if c in r"\.+*?()|[]{}^$#&-~" else c for c in text)


def command(argv, log):
    started = time.monotonic()
    with log.open("w") as output:
        result = subprocess.run(argv, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT)
    return {"argv": argv, "exit_code": result.returncode,
            "seconds": time.monotonic() - started, "log": str(log)}


def source_receipt(package):
    files = capture(["git", "ls-files", "-z", f"crates/{package}",
                     "crates/turbo-dom-layout", "crates/turbo-dom", "Cargo.toml",
                     "Cargo.lock", "crates/turbo-dom-v8/pins.json",
                     "scripts/mutation_sweep.py", "scripts/mutation-sweep-plan.json",
                     ".github/workflows/mutation-sweep.yml"]).split("\0")
    hashes = {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
              for name in files if name}
    return {"commit": capture(["git", "rev-parse", "HEAD"]),
            "tree": capture(["git", "rev-parse", "HEAD^{tree}"]),
            "rustc": capture(["rustc", "--version", "--verbose"]),
            "cargo": capture(["cargo", "--version"]), "files": hashes}


def source_clean():
    changed = capture(["git", "diff", "HEAD", "--name-only"])
    untracked = capture(["git", "ls-files", "--others", "--exclude-standard", "crates"])
    return not changed and not untracked


def outcomes(directory, expected):
    data = json.loads((directory / "mutants.out/outcomes.json").read_text())
    rows = {}
    for row in data["outcomes"]:
        scenario = row["scenario"]
        if not isinstance(scenario, dict) or "Mutant" not in scenario:
            raise ValueError("unexpected baseline or scenario in baseline-skipped run")
        name = scenario["Mutant"]["name"]
        if name in rows:
            raise ValueError("duplicate mutant outcome")
        if row["summary"] not in {"CaughtMutant", "MissedMutant", "Unviable", "Timeout", "Failure", "Success"}:
            raise ValueError("unknown mutation outcome")
        for key in ("log_path", "diff_path"):
            path = directory / "mutants.out" / row[key]
            if not path.is_file() or not path.stat().st_size:
                raise ValueError(f"missing {key} evidence")
        rows[name] = row
    if set(rows) != set(expected) or data["total_mutants"] != len(expected):
        raise ValueError("incomplete or unexpected mutant coverage")
    return rows


def run_job(args):
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    package, features = TARGETS[args.target]
    report = {"target": args.target, "mode": args.mode, "shard": args.shard,
              "shards": args.shards, "complete_scope": False, "ok": False}
    try:
        # Criterion benches do not accept libtest's --test-threads CLI flag.
        os.environ.setdefault("RUST_TEST_THREADS", "2")
        if not source_clean():
            raise ValueError("tracked checkout must be clean before mutation testing")
        if capture(["cargo", "mutants", "--version"]) != f"cargo-mutants {TOOL_VERSION}":
            raise ValueError("unexpected cargo-mutants version")
        before = source_receipt(package)
        write_json(output / "source-before.json", before)
        report["commit"] = before["commit"]
        report["tree"] = before["tree"]
        metadata = capture(["cargo", "metadata", "--locked", "--format-version=1"])
        (output / "cargo-metadata.json").write_text(metadata + "\n")
        archive = os.environ.get("RUSTY_V8_ARCHIVE")
        if archive:
            report["v8_archive_sha256"] = hashlib.sha256(Path(archive).read_bytes()).hexdigest()
        feature_args = ["--features", features] if features else []
        common = ["cargo", "mutants", "--no-config", "-p", package,
                  "-f", target_path(args.target), *feature_args]
        inventory = json.loads(capture([*common, "--list", "--json"]))
        write_json(output / "inventory-all.json", inventory)
        shard, shards = args.shard, args.shards
        if args.mode == "pilot":
            limit = json.loads(PLAN.read_text())["pilot_mutants_per_target"]
            if not isinstance(limit, int) or not 1 <= limit <= 5:
                raise ValueError("pilot limit must remain between one and five")
            shard, shards = 0, max(1, math.ceil(len(inventory) / limit))
        selected = json.loads(capture([*common, "--list", "--json", "--shard", f"{shard}/{shards}"]))
        names = [item["name"] for item in selected]
        if not names:
            raise ValueError("empty shard cannot establish mutation evidence")
        write_json(output / "inventory-selected.json", selected)
        report["all_names"] = [item["name"] for item in inventory]
        report["selected_names"] = names
        # The exact unmutated package/all-targets graph is checked before either skip.
        baseline = command(["cargo", "test", "--locked", "-p", package,
                            *feature_args, "--all-targets"],
                           output / "baseline.log")
        report["baseline"] = baseline
        if baseline["exit_code"]:
            raise ValueError("unmutated all-targets baseline failed")
        flags = ["--in-place", "--no-shuffle", "--baseline", "skip",
                 "--timeout", "300", "--build-timeout", "300", "--jobs", "1",
                 "--cap-lints", "false", "-C", "--locked"]
        lib_dir = output / "lib"
        lib_dir.mkdir()
        report["lib_run"] = command([*common, *flags, "-C", "--lib", "--shard",
                                     f"{shard}/{shards}", "-o", str(lib_dir)], output / "lib.log")
        rows = outcomes(lib_dir, names)
        missed = [name for name, row in rows.items() if row["summary"] == "MissedMutant"]
        if missed:
            # External integration controls may catch a mutant missed by lib tests.
            pattern = "^(?:" + "|".join(map(rust_regex_literal, missed)) + ")$"
            full_dir = output / "all-targets"
            full_dir.mkdir()
            report["all_target_run"] = command([*common, *flags, "-C", "--all-targets",
                                                "--re", pattern, "-o", str(full_dir)], output / "all-targets.log")
            rows.update(outcomes(full_dir, missed))
        report["results"] = {name: row["summary"] for name, row in rows.items()}
        report["unresolved"] = [name for name, row in rows.items()
                                if row["summary"] not in {"CaughtMutant", "Unviable"}]
        after = source_receipt(package)
        write_json(output / "source-after.json", after)
        if before != after or not source_clean():
            raise ValueError("mutation runner did not restore the exact source graph")
        report["ok"] = not report["unresolved"]
        report["classification_note"] = "Unviable includes compiler log and diff; no equivalence exemptions. Timeouts and unknown/missed outcomes fail."
    except (ValueError, OSError, subprocess.SubprocessError, KeyError) as error:
        report["error"] = str(error)
    write_json(output / "summary.json", report)
    return 0 if report["ok"] else 1


def aggregate(directory):
    reports = [json.loads(path.read_text()) for path in Path(directory).rglob("summary.json")]
    if not reports or not all(report.get("ok") for report in reports):
        raise ValueError("missing or failing shard evidence")
    modes = {report["mode"] for report in reports}
    if len(modes) != 1:
        raise ValueError("mixed pilot/full evidence")
    if len({(report["commit"], report["tree"]) for report in reports}) != 1:
        raise ValueError("mixed source revisions")
    counts = {}
    for target in TARGETS:
        group = [report for report in reports if report["target"] == target]
        if not group:
            raise ValueError(f"missing target {target}")
        expected = set(group[0]["all_names"])
        if any(set(report["all_names"]) != expected for report in group):
            raise ValueError("inconsistent target inventories")
        selected = [name for report in group for name in report["selected_names"]]
        if len(selected) != len(set(selected)):
            raise ValueError("overlapping shards")
        if modes == {"full"} and set(selected) != expected:
            raise ValueError(f"incomplete full sweep for {target}")
        counts[target] = {"tested": len(selected), "inventory": len(expected)}
    result = {"ok": True, "mode": next(iter(modes)),
              "complete_scope": modes == {"full"}, "counts": counts,
              "commit": reports[0]["commit"], "tree": reports[0]["tree"]}
    write_json(Path(directory) / "aggregate.json", result)
    print(json.dumps(result))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    mat = sub.add_parser("matrix")
    mat.add_argument("--mode", choices=("", "pilot", "full"), default="")
    run = sub.add_parser("run")
    run.add_argument("--target", choices=TARGETS, required=True)
    run.add_argument("--mode", choices=("pilot", "full"), required=True)
    run.add_argument("--shard", type=int, required=True)
    run.add_argument("--shards", type=int, required=True)
    run.add_argument("--output", required=True)
    agg = sub.add_parser("aggregate")
    agg.add_argument("directory")
    args = parser.parse_args()
    if args.command == "matrix":
        print(json.dumps(matrix(args.mode)))
    elif args.command == "run":
        return run_job(args)
    else:
        aggregate(args.directory)
    return 0


if __name__ == "__main__":
    sys.exit(main())
