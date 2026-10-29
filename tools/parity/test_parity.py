"""Unit self-test of the parity harness driver and its committed data (no addons needed).

  python3 -m unittest discover -s tools/parity -p 'test_*.py'
"""
import contextlib
import importlib.util
import io
import json
import re
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

HERE = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location("parity", HERE / "parity.py")
parity = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(parity)
SHA = re.compile(r"^[0-9a-f]{64}$")


class Workspace:
    """A temporary checkpoint store + output directory for compare/record."""

    def __init__(self, test):
        self.tmp = Path(test.enterContext(tempfile.TemporaryDirectory()))
        self.manifests = self.tmp / "manifests"
        self.out = self.tmp / "out"
        test.enterContext(mock.patch.object(parity, "MANIFESTS", self.manifests))

    def checkpoint(self, name, corpora):
        directory = self.manifests / name
        directory.mkdir(parents=True)
        for corpus, manifest in corpora.items():
            (directory / f"{corpus}.json").write_text(json.dumps(manifest))

    def produce(self, corpora):
        for corpus, manifest in corpora.items():
            (self.out / corpus).mkdir(parents=True, exist_ok=True)
            (self.out / corpus / "MANIFEST.json").write_text(json.dumps(manifest))

    def expectation(self, name, changes, added=None):
        path = self.tmp / f"{name}.json"
        spec = {"id": name, "reason": "test", "changes": changes}
        if added is not None:
            spec["added"] = added
        path.write_text(json.dumps(spec))
        return str(path)

    def compare(self, checkpoint, expect=None):
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout), self_assert_exit() as code:
            parity.compare(SimpleNamespace(out=str(self.out), checkpoint=checkpoint, expect=expect))
        return code.value, stdout.getvalue()


@contextlib.contextmanager
def self_assert_exit():
    holder = SimpleNamespace(value=None)
    try:
        yield holder
    except SystemExit as error:
        holder.value = error.code
    else:
        raise AssertionError("compare() did not exit")


class CompareTest(unittest.TestCase):
    def setUp(self):
        self.ws = Workspace(self)
        self.ws.checkpoint("cp", {"html2pdf": {"a.pdf": "1", "b.pdf": "2"}, "surf": {"s.png": "3"}})

    def test_identical_outputs_pass(self):
        self.ws.produce({"html2pdf": {"a.pdf": "1", "b.pdf": "2"}, "surf": {"s.png": "3"}})
        code, out = self.ws.compare("cp")
        self.assertEqual(code, 0)
        self.assertIn("2 corpora compared against cp: 0 failure(s)", out)

    def test_a_corpus_that_was_not_produced_is_not_compared(self):
        self.ws.produce({"surf": {"s.png": "3"}})
        code, out = self.ws.compare("cp")
        self.assertEqual(code, 0)
        self.assertIn("1 corpora compared", out)

    def test_changed_added_and_removed_outputs_fail(self):
        self.ws.produce({"html2pdf": {"a.pdf": "X", "c.pdf": "4"}})
        code, out = self.ws.compare("cp")
        self.assertEqual(code, 1)
        self.assertIn("UNEXPECTED html2pdf/a.pdf (changed)", out)
        self.assertIn("UNEXPECTED html2pdf/b.pdf (removed)", out)
        self.assertIn("UNEXPECTED html2pdf/c.pdf (added)", out)

    def test_an_expected_change_passes_and_names_its_owner(self):
        self.ws.produce({"html2pdf": {"a.pdf": "X", "b.pdf": "2"}})
        expect = self.ws.expectation("fix-1", {"html2pdf": ["a.pdf"]})
        code, out = self.ws.compare("cp", [expect])
        self.assertEqual(code, 0)
        self.assertIn("expected  html2pdf/a.pdf (changed; fix-1)", out)

    def test_a_stale_expectation_fails(self):
        self.ws.produce({"html2pdf": {"a.pdf": "1", "b.pdf": "2"}})
        expect = self.ws.expectation("fix-1", {"html2pdf": ["a.pdf"]})
        code, out = self.ws.compare("cp", [expect])
        self.assertEqual(code, 1)
        self.assertIn("STALE expectation html2pdf/a.pdf (fix-1) did not change", out)

    def test_an_expectation_for_an_unproduced_corpus_is_not_stale(self):
        self.ws.produce({"html2pdf": {"a.pdf": "1", "b.pdf": "2"}})
        expect = self.ws.expectation("fix-1", {"surf": ["s.png"]})
        self.assertEqual(self.ws.compare("cp", [expect])[0], 0)

    def test_a_declared_added_output_passes(self):
        self.ws.produce({"html2pdf": {"a.pdf": "1", "b.pdf": "2", "c.pdf": "4"}})
        expect = self.ws.expectation("new-input", {}, {"html2pdf": ["c.pdf"]})
        code, out = self.ws.compare("cp", [expect])
        self.assertEqual(code, 0)
        self.assertIn("expected  html2pdf/c.pdf (added; new-input)", out)

    def test_a_declared_added_output_that_is_not_produced_is_stale(self):
        self.ws.produce({"html2pdf": {"a.pdf": "1", "b.pdf": "2"}})
        expect = self.ws.expectation("new-input", {}, {"html2pdf": ["c.pdf"]})
        code, out = self.ws.compare("cp", [expect])
        self.assertEqual(code, 1)
        self.assertIn("STALE expectation html2pdf/c.pdf (new-input) did not change", out)

    def test_an_added_declaration_for_an_existing_output_fails(self):
        self.ws.produce({"html2pdf": {"a.pdf": "X", "b.pdf": "2"}})
        expect = self.ws.expectation("new-input", {}, {"html2pdf": ["a.pdf"]})
        code, out = self.ws.compare("cp", [expect])
        self.assertEqual(code, 1)
        self.assertIn("NOT NEW html2pdf/a.pdf (changed; declared added by new-input)", out)

    def test_unknown_checkpoint_and_no_overlap_exit_with_a_message(self):
        self.ws.produce({"html2pdf": {"a.pdf": "1"}})
        self.assertEqual(self.ws.compare("nope")[0], "compare: unknown checkpoint nope")
        self.ws.checkpoint("other", {"surf-extra": {"x.png": "1"}})
        self.assertIn("no produced corpus", self.ws.compare("other")[0])


class RecordAndRunTest(unittest.TestCase):
    def test_record_keeps_added_outputs(self):
        ws = Workspace(self)
        ws.produce({"html2pdf": {"a.pdf": "1", "c.pdf": "4"}})
        with contextlib.redirect_stdout(io.StringIO()):
            parity.record(SimpleNamespace(out=str(ws.out), checkpoint="next"))
        self.assertEqual(json.loads((ws.manifests / "next/html2pdf.json").read_text()), {"a.pdf": "1", "c.pdf": "4"})

    def test_record_copies_every_produced_corpus_manifest(self):
        ws = Workspace(self)
        ws.produce({"surf": {"s.png": "3"}, "not-a-corpus": {"x": "1"}})
        with contextlib.redirect_stdout(io.StringIO()):
            parity.record(SimpleNamespace(out=str(ws.out), checkpoint="new"))
        self.assertEqual(sorted(p.name for p in (ws.manifests / "new").iterdir()), ["surf.json"])
        self.assertEqual(json.loads((ws.manifests / "new/surf.json").read_text()), {"s.png": "3"})

    def test_run_requires_an_addon(self):
        args = SimpleNamespace(out="o", html2pdf=None, html2pdf_svg=None, surf=None)
        with self.assertRaises(SystemExit) as raised:
            parity.run(args)
        self.assertIn("pass at least one addon", str(raised.exception.code))

    def test_run_invokes_each_corpus_script_for_its_addon(self):
        args = SimpleNamespace(out="/o", html2pdf=None, html2pdf_svg=None, surf="/a/surf.node")
        with mock.patch.object(parity.subprocess, "run") as run:
            parity.run(args)
        scripts = [call.args[0][1] for call in run.call_args_list]
        self.assertEqual(scripts, [str(parity.BIN / "corpus-surf.mjs"), str(parity.BIN / "corpus-surf-extra.mjs")])
        env = run.call_args_list[0].kwargs["env"]
        self.assertEqual((env["TZ"], env["LANG"]), ("UTC", "C"))


class CommittedDataTest(unittest.TestCase):
    def test_every_corpus_script_exists(self):
        for script, _ in parity.CORPORA.values():
            self.assertTrue((parity.BIN / script).is_file(), script)

    def test_checkpoint_manifests_map_outputs_to_sha256(self):
        manifests = sorted(parity.MANIFESTS.glob("*/*.json"))
        self.assertTrue(manifests)
        for path in manifests:
            self.assertIn(path.stem, parity.CORPORA, path)
            manifest = json.loads(path.read_text())
            self.assertTrue(manifest, path)
            for key, digest in manifest.items():
                self.assertRegex(digest, SHA, f"{path}: {key}")

    def test_expected_diff_files_are_well_formed_and_name_recorded_outputs(self):
        recorded = {}
        for path in parity.MANIFESTS.glob("*/*.json"):
            recorded.setdefault(path.stem, set()).update(json.loads(path.read_text()))
        files = sorted((HERE / "expected").glob("*.json"))
        self.assertTrue(files)
        for path in files:
            spec = json.loads(path.read_text())
            self.assertTrue(spec.get("id") and spec.get("reason"), path)
            for corpus, keys in spec["changes"].items():
                self.assertIn(corpus, parity.CORPORA, path)
                for key in keys:
                    self.assertIn(key, recorded.get(corpus, ()), f"{path}: {corpus}/{key}")
            # Added outputs belong to new inputs, so no recorded checkpoint needs them;
            # compare itself rejects one that the compared checkpoint already has.
            for corpus, keys in spec.get("added", {}).items():
                self.assertIn(corpus, parity.CORPORA, path)
                for key in keys:
                    self.assertRegex(key, r"\S", f"{path}: {corpus}/{key!r}")


if __name__ == "__main__":
    unittest.main()
