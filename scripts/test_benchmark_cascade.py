import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "cascade_benchmark", Path(__file__).with_name("benchmark-cascade.py")
)
BENCH = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCH)


class CascadeBenchmarkTests(unittest.TestCase):
    def inputs(self, directory):
        root = Path(directory)
        for label in ["baseline", "candidate"]:
            (root / label).write_bytes(label.encode())
        return SimpleNamespace(baseline=root / "baseline", candidate=root / "candidate",
                               iterations=10, rounds=3)

    def test_balanced_blocks_preserve_output_and_compute_paired_improvement(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.inputs(directory)
            labels = []

            def sample(binary, iterations):
                label = binary.name
                labels.append(label)
                duration = 100 if label == "baseline" else 80
                return {"layout": {"name": "layout", "ns": duration,
                                   "iterations": iterations, "digest": "unchanged"}}, duration

            with patch.object(BENCH, "run", side_effect=sample), contextlib.redirect_stdout(io.StringIO()):
                report = BENCH.compare(args)
            self.assertEqual(labels[:8], ["baseline", "candidate", "candidate", "baseline",
                                          "candidate", "baseline", "baseline", "candidate"])
            self.assertAlmostEqual(report["results"]["layout"]["paired_median_improvement_percent"], 20)
            self.assertAlmostEqual(report["paired_median_cpu_improvement_percent"], 20)
            self.assertEqual(len(report["samples_ns_per_iteration"]["baseline"]["layout"]), 3)

    def test_accuracy_drift_fails_instead_of_reporting_a_speedup(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.inputs(directory)

            def sample(binary, iterations):
                return {"layout": {"name": "layout", "ns": 1,
                                   "iterations": iterations, "digest": binary.name}}, 1

            with patch.object(BENCH, "run", side_effect=sample):
                with self.assertRaisesRegex(ValueError, "output digests differ"):
                    BENCH.compare(args)


if __name__ == "__main__":
    unittest.main()
