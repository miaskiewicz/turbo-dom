"""Self-tests for check-baseline-red.py's cargo log parsing."""
import importlib.util
import unittest
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "check_baseline_red", Path(__file__).resolve().parent / "check-baseline-red.py")
CHECK = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(CHECK)

LOG = (
    "     Running tests/layout_flex.rs (target/debug/deps/layout_flex-0123456789abcdef)\n"
    "test wraps ... FAILED\n"
    "test fits ... ok\n"
)
# CI sets CARGO_TERM_COLOR=always: cargo colors its status words (bold green "Running").
COLORED = LOG.replace("     Running", "\x1b[1m\x1b[92m     Running\x1b[0m")


class ParseTests(unittest.TestCase):
    def test_plain_log_attributes_tests_to_their_binary(self):
        self.assertEqual(CHECK.parse(LOG), {
            ("layout_flex<tests/layout_flex.rs>", "wraps"): "FAILED",
            ("layout_flex<tests/layout_flex.rs>", "fits"): "ok",
        })

    def test_colored_cargo_status_lines_attribute_tests_like_plain_ones(self):
        self.assertEqual(CHECK.parse(COLORED), CHECK.parse(LOG))


if __name__ == "__main__":
    unittest.main()
