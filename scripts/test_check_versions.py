"""Unit tests for scripts/check-versions.py's Cargo.lock and README snippet checks.

  python3 -m unittest discover -s scripts -p 'test_*.py'
"""
import importlib.util
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location("check_versions", HERE / "check-versions.py")
cv = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(cv)


def lock(*packages):
    blocks = ["version = 4\n"]
    for name, version, source in packages:
        block = f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
        if source:
            block += f'source = "{source}"\n'
        blocks.append(block)
    return "\n".join(blocks)


GIT = "git+https://github.com/miaskiewicz/turbo-dom?rev=a#a"
OURS = [("turbo-dom-parser", "0.8.0", None), ("turbo-dom", "0.8.0", None),
        ("turbo-dom-layout", "0.8.0", None), ("turbo-dom-raster", "0.8.0", None),
        ("turbo-dom-browser-env", "0.8.0", None), ("turbo-dom-v8", "150.4.1", GIT),
        ("itertools", "0.13.0", "registry+https://github.com/rust-lang/crates.io-index")]


class LockTests(unittest.TestCase):
    def test_a_current_lock_passes(self):
        self.assertEqual(cv.lock_errors(lock(*OURS), "0.8.0", "150.4.1"), [])

    def test_a_stale_workspace_crate_fails(self):
        # The 0.8.0 release PR's lock still had the root parser crate at 0.7.1.
        stale = [("turbo-dom-parser", "0.7.1", None)] + OURS[1:]
        self.assertEqual(cv.lock_errors(lock(*stale), "0.8.0", "150.4.1"),
                         ["Cargo.lock locks turbo-dom-parser at ['0.7.1'], expected 0.8.0"])

    def test_the_fork_must_be_locked_at_the_pins_version(self):
        self.assertEqual(cv.lock_errors(lock(*OURS), "0.8.0", "150.4.2"),
                         ["Cargo.lock locks turbo-dom-v8 at ['150.4.1'], expected 150.4.2"])

    def test_a_missing_or_duplicated_package_fails(self):
        missing = [package for package in OURS if package[0] != "turbo-dom-raster"]
        doubled = OURS + [("turbo-dom-v8", "150.4.0", "registry+x")]
        self.assertEqual(cv.lock_errors(lock(*missing), "0.8.0", "150.4.1"),
                         ["Cargo.lock locks turbo-dom-raster at nothing, expected 0.8.0"])
        self.assertEqual(cv.lock_errors(lock(*doubled), "0.8.0", "150.4.1"),
                         ["Cargo.lock locks turbo-dom-v8 at ['150.4.1', '150.4.0'], expected 150.4.1"])


class SnippetTests(unittest.TestCase):
    def test_snippets_naming_the_release_pass(self):
        text = ('turbo-dom = "0.8"\nturbo-dom-layout = { version = "0.8.0", default-features = false }\n'
                'cargo add turbo-dom        # turbo-dom = "0.8"; MSRV 1.89\n'
                'v8 = { package = "turbo-dom-v8", version = "~150.4" }\n')
        self.assertEqual(cv.snippet_errors("README.md", text, "0.8.0"), [])

    def test_a_previous_release_snippet_fails(self):
        text = 'x\nturbo-dom-raster = "0.8"\nturbo-dom-browser-env = { version = "0.7", features = ["test-support"] }\n'
        self.assertEqual(cv.snippet_errors("crates/x/README.md", text, "0.8.0"),
                         ['crates/x/README.md:3: turbo-dom-browser-env = "0.7", release is 0.8'])


if __name__ == "__main__":
    unittest.main()
