#!/usr/bin/env python3
"""Build identical cascade harnesses at two Git refs, then compare speed and output.

Only a newly created scratch worktree is edited. The caller's checkout is untouched.
The output directory must not exist; logs, binaries and JSON remain there for review.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[1]
EXAMPLE = Path("crates/turbo-dom-layout/examples/cascade_perf.rs")


def git(*args):
    return subprocess.check_output(["git", "-C", str(REPO), *args], text=True).strip()


def build(source, target, ref, label, args):
    subprocess.run(["git", "-C", str(source), "checkout", "--detach", ref], check=True)
    harness = source / EXAMPLE
    original = harness.read_bytes() if harness.exists() else None
    harness.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(REPO / EXAMPLE, harness)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS=str(args.jobs))
    try:
        command = ["cargo", "+" + args.toolchain, "build", "-p", "turbo-dom-layout",
                   "--example", "cascade_perf", "--release", "--locked"]
        with (args.out / f"{label}-build.log").open("w") as log:
            subprocess.run(command, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        shutil.copy2(target / "release/examples/cascade_perf", args.out / label)
        if label == "candidate" and args.accuracy_gates:
            command = ["cargo", "+" + args.toolchain, "test", "-p", "turbo-dom-layout",
                       "--all-features", "--locked"]
            with (args.out / "candidate-accuracy.log").open("w") as log:
                subprocess.run(command, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    finally:
        if original is None:
            harness.unlink()
        else:
            harness.write_bytes(original)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline")
    parser.add_argument("candidate")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--toolchain", default="1.98.0")
    parser.add_argument("--target-dir", type=Path, help="optional reusable Cargo build cache")
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument("--iterations", type=int, default=100)
    parser.add_argument("--rounds", type=int, default=15)
    parser.add_argument("--accuracy-gates", action="store_true")
    args = parser.parse_args()
    if args.jobs < 1 or args.iterations < 1 or args.rounds < 3:
        parser.error("positive jobs/iterations and at least three rounds are required")
    args.out = args.out.resolve()
    harness_hashes = {
        str(path): hashlib.sha256((REPO / path).read_bytes()).hexdigest()
        for path in [EXAMPLE, Path("scripts/benchmark-cascade.py"),
                     Path("scripts/benchmark-cascade-refs.py")]
    }
    refs = {label: git("rev-parse", ref + "^{commit}")
            for label, ref in [("baseline", args.baseline), ("candidate", args.candidate)]}
    args.out.mkdir(parents=True, exist_ok=False)
    source = args.out / "source"
    target = args.target_dir.resolve() if args.target_dir else args.out / "target"
    git("worktree", "add", "--detach", str(source), refs["baseline"])
    try:
        for label, ref in refs.items():
            build(source, target, ref, label, args)
    finally:
        git("worktree", "remove", str(source))
    # Run after all compilation/tests finish so measurement has no competing build.
    subprocess.run(["python3", str(REPO / "scripts/benchmark-cascade.py"),
                    "--baseline", str(args.out / "baseline"),
                    "--candidate", str(args.out / "candidate"),
                    "--iterations", str(args.iterations), "--rounds", str(args.rounds),
                    "--out", str(args.out / "comparison.json")], check=True)
    report = json.loads((args.out / "comparison.json").read_text())
    report["revisions"] = refs
    report["toolchain"] = subprocess.check_output(
        ["rustc", "+" + args.toolchain, "--version"], text=True).strip()
    report["harness_revision"] = git("rev-parse", "HEAD")
    report["harness_sha256"] = harness_hashes
    report["accuracy_gates"] = args.accuracy_gates
    (args.out / "comparison.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
