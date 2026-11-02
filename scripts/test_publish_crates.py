"""Unit tests for scripts/publish-crates.py (no network, no cargo).

  python3 -m unittest discover -s scripts -p 'test_*.py'
"""
import importlib.util
import json
import tomllib
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location("publish_crates", HERE / "publish-crates.py")
pc = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(pc)


def fake(responses):
    calls = []

    def fetch(url):
        calls.append(url)
        value = responses.get(url)
        return value.pop(0) if isinstance(value, list) else value

    fetch.calls = calls
    return fetch


class IndexTest(unittest.TestCase):
    def test_sparse_index_paths(self):
        self.assertEqual(pc.index_path("a"), "1/a")
        self.assertEqual(pc.index_path("ab"), "2/ab")
        self.assertEqual(pc.index_path("abc"), "3/a/abc")
        self.assertEqual(pc.index_path("turbo-dom"), "tu/rb/turbo-dom")
        self.assertEqual(pc.index_path("Turbo-DOM-v8"), "tu/rb/turbo-dom-v8")

    def test_indexed_versions_reads_every_line_and_handles_a_missing_crate(self):
        url = "https://index.crates.io/tu/rb/turbo-dom"
        body = b'{"name":"turbo-dom","vers":"0.5.8"}\n{"name":"turbo-dom","vers":"0.6.0"}\n'
        self.assertEqual(pc.indexed_versions("turbo-dom", fake({url: body})), {"0.5.8", "0.6.0"})
        self.assertEqual(pc.indexed_versions("turbo-dom", fake({})), set())

    def test_wait_index_polls_until_listed(self):
        url = "https://index.crates.io/tu/rb/turbo-dom"
        listed = b'{"vers":"0.6.0"}\n'
        fetch = fake({url: [None, b'{"vers":"0.5.8"}\n', listed]})
        sleeps = []
        pc.wait_index("turbo-dom", "0.6.0", attempts=5, fetch=fetch, sleep=sleeps.append)
        self.assertEqual((len(fetch.calls), len(sleeps)), (3, 2))

    def test_wait_index_gives_up(self):
        with self.assertRaises(RuntimeError):
            pc.wait_index("turbo-dom", "0.6.0", attempts=3, fetch=fake({}), sleep=lambda _: None)


class PublishedTest(unittest.TestCase):
    API = "https://crates.io/api/v1/crates/turbo-dom/0.6.0"

    def test_absent_and_present(self):
        self.assertFalse(pc.published("turbo-dom", "0.6.0", fake({})))
        body = json.dumps({"version": {"crate": "turbo-dom", "num": "0.6.0"}}).encode()
        self.assertTrue(pc.published("turbo-dom", "0.6.0", fake({self.API: body})))

    def test_a_mismatched_answer_is_fatal(self):
        body = json.dumps({"version": {"crate": "turbo-dom", "num": "0.5.8"}}).encode()
        with self.assertRaises(ValueError):
            pc.published("turbo-dom", "0.6.0", fake({self.API: body}))


class PatchTest(unittest.TestCase):
    def test_strip_the_root_manifest_patch_keeps_everything_else(self):
        text = (pc.REPO / "Cargo.toml").read_text()
        before = tomllib.loads(text)
        stripped = pc.strip_patch_section(text)
        after = tomllib.loads(stripped)
        before.pop("patch", None)
        self.assertEqual(after, before)
        self.assertNotIn("turbo-dom-v8 candidate", stripped.split("[profile.profiling]")[-1])

    def test_strip_a_middle_table_and_its_comment(self):
        text = '[a]\nx = 1\n\n# pre-publish only\n[patch.crates-io]\nv = { git = "g" }\n\n[b]\ny = 2\n'
        stripped = pc.strip_patch_section(text)
        self.assertEqual(tomllib.loads(stripped), {"a": {"x": 1}, "b": {"y": 2}})
        self.assertNotIn("pre-publish", stripped)

    def test_no_patch_is_a_no_op(self):
        self.assertEqual(pc.strip_patch_section("[a]\nx = 1\n"), "[a]\nx = 1\n")

    def test_publish_order_follows_the_dependency_graph(self):
        self.assertEqual(pc.ORDER, ["turbo-dom", "turbo-dom-layout", "turbo-dom-raster", "turbo-dom-browser-env"])
        for position, name in enumerate(pc.ORDER):
            deps = tomllib.loads((pc.REPO / "crates" / name / "Cargo.toml").read_text())["dependencies"]
            for dep in deps.values():
                if isinstance(dep, dict) and "path" in dep:
                    sibling = dep.get("package", Path(dep["path"]).name)
                    self.assertIn(sibling, pc.ORDER[:position], f"{name} needs {sibling} published first")


if __name__ == "__main__":
    unittest.main()
