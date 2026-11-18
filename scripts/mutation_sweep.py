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
PRIMARY_FILTERS = {
    "event_handlers": "handler",
    "worker_scope": "worker_scope::",
    "trusted_types": "trusted_types::",
}

PRIMARY_CARGO_TARGETS = {
    "paint_png": ["--lib", "--test", "multicol_rule_chrome", "--test", "paint_memory", "--test", "png_rule_progress"],
    "paint_svg": ["--lib", "--test", "multicol_rule_chrome", "--test", "svg_rule_progress"],
}


def primary_cargo_targets(target):
    return PRIMARY_CARGO_TARGETS.get(target, ["--lib"])


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def capture(argv):
    return subprocess.check_output(argv, cwd=ROOT, text=True).strip()


def target_path(target):
    package, _ = TARGETS[target]
    return f"crates/{package}/src/{target}.rs"


def matrix(mode, target=""):
    plan = json.loads(PLAN.read_text())
    mode = mode or plan["mode"]
    if mode not in {"pilot", "full"}:
        raise ValueError("mode must be pilot or full")
    if set(plan["full_shards"]) != set(TARGETS):
        raise ValueError("all five targets must be present")
    if target not in ("", "all", *TARGETS):
        raise ValueError("unknown mutation target")
    selected_targets = TARGETS if target in ("", "all") else (target,)
    jobs = []
    for target in selected_targets:
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



def selection_diff(inventory, names, root=ROOT):
    """A source-matching line mask, not a claimed VCS change or source edit."""
    by_name = {item["name"]: item for item in inventory}
    if len(by_name) != len(inventory) or not names or len(set(names)) != len(names):
        raise ValueError("duplicate or empty exact replay selection")
    lines_by_file = {}
    for name in names:
        item = by_name[name]
        path, line = item["file"], item["span"]["start"]["line"]
        if Path(path).is_absolute() or ".." in Path(path).parts or type(line) is not int or line < 1:
            raise ValueError("invalid replay source location")
        lines_by_file.setdefault(path, set()).add(line)
    result = []
    for path, selected in sorted(lines_by_file.items()):
        source = (root / path).read_text().splitlines()
        result.extend([f"--- a/{path}", f"+++ b/{path}"])
        for line in sorted(selected):
            if line > len(source):
                raise ValueError("replay source location exceeds current file")
            # Deleted lines include their neighbors in cargo-mutants' diff
            # filter. An addition-only mask selects precisely this new line.
            result.extend([f"@@ -{line - 1},0 +{line},1 @@", "+" + source[line - 1]])
    return "\n".join(result) + "\n"


def exact_replay_filters(common, inventory, names, output):
    # cargo-mutants 27.1.0's StructField visitor bypasses --re. Its later
    # --in-diff pass filters every genre; validate the resulting exact set.
    mask = output / "fallback-selection.diff"
    mask.write_text(selection_diff(inventory, names))
    pattern = "^(?:" + "|".join(map(rust_regex_literal, names)) + ")$"
    filters = ["--re", pattern, "--in-diff", str(mask)]
    selected = json.loads(capture([*common, "--list", "--json", *filters]))
    actual = [item["name"] for item in selected]
    write_json(output / "fallback-inventory-selected.json", selected)
    if len(actual) != len(set(actual)) or set(actual) != set(names):
        raise ValueError("exact replay inventory contains missing or unexpected mutants")
    return filters


def mutation_flags():
    # --in-place forces serial mutation; cargo-mutants rejects even --jobs 1.
    return ["--in-place", "--no-shuffle", "--baseline", "skip",
            "--timeout", "300", "--build-timeout", "300",
            "--cap-lints", "false", "-C", "--locked"]


def full_fallback_timeout(test_phase):
    if not isinstance(test_phase, dict) or test_phase.get("phase") != "Test" or test_phase.get("exit_code") != 0:
        raise ValueError("fallback requires a successful separate unmutated Test phase")
    seconds = test_phase.get("seconds")
    if type(seconds) not in (int, float) or not math.isfinite(seconds) or seconds <= 0:
        raise ValueError("fallback requires a finite positive observed test duration")
    budget = max(300, math.ceil(2 * seconds + 30))
    if budget > 900:
        raise ValueError("observed Test phase exceeds the reviewed 900-second fallback ceiling")
    return budget


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



def validate_inventory_count(target, inventory):
    counts = json.loads(PLAN.read_text())["expected_inventory_counts"]
    if set(counts) != set(TARGETS) or any(type(value) is not int or value <= 0 for value in counts.values()):
        raise ValueError("invalid reviewed inventory counts")
    names = [item["name"] for item in inventory]
    if len(names) != len(set(names)) or len(names) != counts[target]:
        raise ValueError(f"inventory mismatch for {target}: reviewed {counts[target]}, actual {len(names)}")


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
        report["actual_inventory_count"] = len(inventory)
        report["inventory_reference_commit"] = json.loads(PLAN.read_text())["inventory_reference_commit"]
        validate_inventory_count(args.target, inventory)
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
        baseline_argv = ["cargo", "test", "--locked", "-p", package,
                         *feature_args, "--all-targets"]
        build_phase = command([*baseline_argv, "--no-run"], output / "baseline-build.log")
        build_phase["phase"] = "Build"
        report["baseline_build"] = build_phase
        if build_phase["exit_code"]:
            raise ValueError("unmutated all-targets Build phase failed")
        test_phase = command(baseline_argv, output / "baseline-test.log")
        test_phase["phase"] = "Test"
        report["baseline_test"] = test_phase
        if test_phase["exit_code"]:
            raise ValueError("unmutated all-targets Test phase failed")
        fallback_budget = full_fallback_timeout(test_phase)
        report["full_fallback_timeout_seconds"] = fallback_budget
        primary_filter = PRIMARY_FILTERS.get(args.target)
        primary_args = [primary_filter] if primary_filter else []
        primary_targets = primary_cargo_targets(args.target)
        primary_baseline = command(["cargo", "test", "--locked", "-p", package,
                                    *feature_args, *primary_targets, *primary_args],
                                   output / "primary-baseline.log")
        report["primary_baseline"] = primary_baseline
        report["primary_cargo_targets"] = primary_targets
        if primary_baseline["exit_code"]:
            raise ValueError("exact primary baseline failed")
        passed = sum(int(value) for value in re.findall(
            r"test result:\s+ok\.\s+(\d+) passed;",
            (output / "primary-baseline.log").read_text()))
        report["primary_baseline_passed_tests"] = passed
        if not passed:
            raise ValueError("filtered baseline executed zero tests")
        flags = mutation_flags()
        lib_dir = output / "lib"
        lib_dir.mkdir()
        primary_target_flags = [flag for target_arg in primary_targets for flag in ("-C", target_arg)]
        report["lib_run"] = command([*common, *flags, *primary_target_flags, "--shard",
                                     f"{shard}/{shards}", "-o", str(lib_dir),
                                     *( ["--", *primary_args] if primary_args else [])], output / "lib.log")
        rows = outcomes(lib_dir, names)
        after_primary = source_receipt(package)
        write_json(output / "source-after-primary.json", after_primary)
        if before != after_primary or not source_clean():
            raise ValueError("primary mutation runner did not restore the exact source graph")
        report["primary_phase_complete"] = True
        primary_results = {name: row["summary"] for name, row in rows.items()}
        write_json(output / "primary-summary.json", {
            "primary_phase_complete": True, "complete_scope": False,
            "commit": before["commit"], "tree": before["tree"],
            "target": args.target, "mode": args.mode,
            "shard": args.shard, "shards": args.shards,
            "all_names": report["all_names"], "selected_names": names,
            "results": primary_results,
            "acceptance_passed": all(value in {"CaughtMutant", "Unviable"} for value in primary_results.values()),
        })
        missed = [name for name, row in rows.items() if row["summary"] == "MissedMutant"]
        if missed:
            # External integration controls may catch a mutant missed by lib tests.
            replay_filters = exact_replay_filters(common, inventory, missed, output)
            full_dir = output / "all-targets"
            full_dir.mkdir()
            full_flags = flags.copy()
            full_flags[full_flags.index("--timeout") + 1] = str(fallback_budget)
            report["all_target_run"] = command([*common, *full_flags, "-C", "--all-targets",
                                                *replay_filters, "-o", str(full_dir)], output / "all-targets.log")
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


def aggregate(directory, target=""):
    reports = [json.loads(path.read_text()) for path in Path(directory).rglob("summary.json")]
    if not reports or not all(report.get("ok") for report in reports):
        raise ValueError("missing or failing shard evidence")
    modes = {report["mode"] for report in reports}
    if len(modes) != 1:
        raise ValueError("mixed pilot/full evidence")
    if len({(report["commit"], report["tree"]) for report in reports}) != 1:
        raise ValueError("mixed source revisions")
    if target not in ("", "all", *TARGETS):
        raise ValueError("unknown mutation target")
    selected_targets = TARGETS if target in ("", "all") else (target,)
    if {report["target"] for report in reports} != set(selected_targets):
        raise ValueError("missing or unexpected target scope")
    counts = {}
    for target in selected_targets:
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
              "complete_scope": modes == {"full"} and set(selected_targets) == set(TARGETS),
              "requested_targets": list(selected_targets), "counts": counts,
              "commit": reports[0]["commit"], "tree": reports[0]["tree"]}
    write_json(Path(directory) / "aggregate.json", result)
    print(json.dumps(result))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    mat = sub.add_parser("matrix")
    mat.add_argument("--mode", choices=("", "pilot", "full"), default="")
    mat.add_argument("--target", choices=("", "all", *TARGETS), default="")
    run = sub.add_parser("run")
    run.add_argument("--target", choices=TARGETS, required=True)
    run.add_argument("--mode", choices=("pilot", "full"), required=True)
    run.add_argument("--shard", type=int, required=True)
    run.add_argument("--shards", type=int, required=True)
    run.add_argument("--output", required=True)
    agg = sub.add_parser("aggregate")
    agg.add_argument("directory")
    agg.add_argument("--target", choices=("", "all", *TARGETS), default="")
    args = parser.parse_args()
    if args.command == "matrix":
        print(json.dumps(matrix(args.mode, args.target)))
    elif args.command == "run":
        return run_job(args)
    else:
        aggregate(args.directory, args.target)
    return 0


if __name__ == "__main__":
    sys.exit(main())
