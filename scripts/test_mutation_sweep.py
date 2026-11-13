"""Fail-closed checks for #487 shard evidence; no cargo or network calls."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

SPEC = importlib.util.spec_from_file_location("mutation_sweep", Path(__file__).with_name("mutation_sweep.py"))
ms = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ms)


class MutationEvidenceTests(unittest.TestCase):
    def write_outcomes(self, root, names, summaries=None):
        directory = root / "mutants.out"
        directory.mkdir()
        rows = []
        for index, name in enumerate(names):
            log, diff = f"{index}.log", f"{index}.diff"
            (directory / log).write_text("actual compiler/test output")
            (directory / diff).write_text("actual changed source")
            rows.append({"scenario": {"Mutant": {"name": name}},
                         "summary": (summaries or {}).get(name, "CaughtMutant"),
                         "log_path": log, "diff_path": diff})
        (directory / "outcomes.json").write_text(json.dumps({"outcomes": rows, "total_mutants": len(rows)}))

    def test_missing_mutant_cannot_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.write_outcomes(root, ["one"])
            with self.assertRaisesRegex(ValueError, "coverage"):
                ms.outcomes(root, ["one", "two"])

    def test_unrecognized_outcome_cannot_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.write_outcomes(root, ["one"], {"one": "IgnoredEquivalent"})
            with self.assertRaisesRegex(ValueError, "unknown"):
                ms.outcomes(root, ["one"])

    def test_mutant_without_raw_diff_cannot_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.write_outcomes(root, ["one"])
            (root / "mutants.out/0.diff").unlink()
            with self.assertRaisesRegex(ValueError, "evidence"):
                ms.outcomes(root, ["one"])

    def test_full_matrix_contains_every_target_and_is_bounded(self):
        jobs = ms.matrix("full")["include"]
        self.assertEqual(set(ms.TARGETS), {job["target"] for job in jobs})
        self.assertLessEqual(len(jobs), 32)
        self.assertEqual(len(jobs), len({job["id"] for job in jobs}))

    def test_in_place_uses_implicit_serial_execution(self):
        self.assertIn("--in-place", ms.mutation_flags())
        self.assertNotIn("--jobs", ms.mutation_flags())

    def run_fake_job(self, primary_count):
        calls = []

        def capture(argv):
            if argv == ["cargo", "mutants", "--version"]:
                return "cargo-mutants 27.1.0"
            if "--list" in argv:
                return '[{"name":"one"}]'
            if "metadata" in argv:
                return "{}"
            self.fail(f"unexpected command {argv}")

        def command(argv, log):
            calls.append((argv, log.name))
            log.write_text(f"test result: ok. {primary_count} passed; 0 failed;" if log.name == "primary-baseline.log"
                           else "actual command output")
            if log.name in {"lib.log", "all-targets.log"}:
                directory = log.parent / ("lib" if log.name == "lib.log" else "all-targets")
                summary = "MissedMutant" if log.name == "lib.log" else "CaughtMutant"
                self.write_outcomes(directory, ["one"], {"one": summary})
            return {"argv": argv, "exit_code": 0, "seconds": 0}

        with tempfile.TemporaryDirectory() as temp:
            args = SimpleNamespace(output=temp, target="worker_scope", mode="full", shard=0, shards=1)
            with patch.object(ms, "capture", capture), patch.object(ms, "command", command), \
                    patch.object(ms, "source_clean", return_value=True), \
                    patch.object(ms, "source_receipt", return_value={"commit":"one", "tree":"one"}), \
                    patch.dict(ms.os.environ, {}, clear=True):
                result = ms.run_job(args)
            report = json.loads((Path(temp) / "summary.json").read_text())
        return result, calls, report

    def test_browser_primary_filters_do_not_filter_full_fallback(self):
        result, calls, _ = self.run_fake_job(11)
        self.assertEqual(result, 0)
        primary = next(argv for argv, name in calls if name == "lib.log")
        fallback = next(argv for argv, name in calls if name == "all-targets.log")
        self.assertIn("worker_scope::", primary)
        self.assertNotIn("worker_scope::", fallback)
        self.assertIn("--all-targets", fallback)

    def test_zero_test_filtered_baseline_stops_before_mutating(self):
        result, calls, report = self.run_fake_job(0)
        self.assertEqual(result, 1)
        self.assertIn("zero tests", report["error"])
        self.assertFalse(any(name == "lib.log" for _, name in calls))

    def test_partial_full_artifacts_cannot_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for target in ms.TARGETS:
                directory = root / target
                directory.mkdir()
                ms.write_json(directory / "summary.json", {
                    "ok": True, "mode": "full", "commit": "one", "tree": "one",
                    "target": target, "all_names": [target + "1", target + "2"],
                    "selected_names": [target + "1"],
                })
            with self.assertRaisesRegex(ValueError, "incomplete full"):
                ms.aggregate(root)

    def test_rust_regex_literal_does_not_escape_spaces(self):
        self.assertEqual(ms.rust_regex_literal("a.rs: replace foo -> bool with true"),
                         r"a\.rs: replace foo \-> bool with true")


if __name__ == "__main__":
    unittest.main()
