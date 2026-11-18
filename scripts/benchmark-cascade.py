#!/usr/bin/env python3
"""Compare two release builds of the cascade_perf example, alternating run order.

Build the identical example at both revisions, then pass their binary paths.
No speed threshold is asserted: inspect paired timings and identical output hashes.
"""
import argparse
import hashlib
import json
import platform
import resource
import statistics
import subprocess
from pathlib import Path


def run(binary, iterations):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    output = subprocess.check_output([str(binary.resolve()), str(iterations)], text=True)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    return {row["name"]: row for row in map(json.loads, output.splitlines())}, cpu


def compare(args):
    samples = {"baseline": {}, "candidate": {}}
    cpu_samples = {"baseline": [], "candidate": []}
    digests = {}
    for round_index in range(args.rounds):
        order = ["baseline", "candidate", "candidate", "baseline"]
        if round_index % 2:
            order = ["candidate", "baseline", "baseline", "candidate"]
        block = {"baseline": {}, "candidate": {}}
        cpu_block = {"baseline": [], "candidate": []}
        for label in order:
            rows, cpu = run(getattr(args, label), args.iterations)
            cpu_block[label].append(cpu)
            current = {name: row["digest"] for name, row in rows.items()}
            if digests and current != digests:
                raise ValueError("workload names or complete output digests differ")
            digests = current
            for name, row in rows.items():
                block[label].setdefault(name, []).append(row["ns"] / row["iterations"])
        for label in samples:
            cpu_samples[label].append(statistics.mean(cpu_block[label]))
            for name, values in block[label].items():
                samples[label].setdefault(name, []).append(statistics.mean(values))
        print(f"completed block {round_index + 1}/{args.rounds}", flush=True)
    results = {}
    for name in digests:
        baseline = samples["baseline"][name]
        candidate = samples["candidate"][name]
        ratios = [b / a for a, b in zip(baseline, candidate)]
        results[name] = {
            "baseline_median_ms": statistics.median(baseline) / 1e6,
            "candidate_median_ms": statistics.median(candidate) / 1e6,
            "paired_median_improvement_percent": (1 - statistics.median(ratios)) * 100,
            "paired_ratios": ratios,
            "digest": digests[name],
        }
    return {
        "platform": platform.platform(),
        "rounds": args.rounds,
        "iterations": args.iterations,
        "order": "alternating ABBA/BAAB blocks; samples average each binary's two runs",
        "cpu_samples_seconds": cpu_samples,
        "paired_median_cpu_improvement_percent": (1 - statistics.median(
            b / a for a, b in zip(cpu_samples["baseline"], cpu_samples["candidate"])
        )) * 100,
        "binary_sha256": {
            label: hashlib.sha256(getattr(args, label).read_bytes()).hexdigest()
            for label in samples
        },
        "results": results,
        "samples_ns_per_iteration": samples,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--iterations", type=int, default=40)
    parser.add_argument("--rounds", type=int, default=11)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    if args.rounds < 3 or args.iterations < 1:
        parser.error("use at least three rounds and positive iterations")
    report = compare(args)
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    for name, result in report["results"].items():
        print(f"{name}: {result['paired_median_improvement_percent']:+.2f}%")
