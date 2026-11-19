"""Self-tests for generate.py: python3 -m unittest discover -s crates/turbo-dom-rav1d/scripts -p 'test_*.py'"""

import json
import tempfile
import unittest
from pathlib import Path

import generate

PINS = json.loads(generate.PINS_PATH.read_text())

UPSTREAM_MANIFEST = """[package]
edition = "2021"
rust-version = "1.79"
name = "rav1d"
version = "1.1.0"
description = "Rust port of the dav1d AV1 decoder"
license = "BSD-2-Clause"
repository = "https://github.com/memorysafety/rav1d"

[lib]
name = "rav1d"
path = "lib.rs"
"""


class UnwindAbi(unittest.TestCase):
    def test_every_c_abi_unwinds(self):
        source = 'pub unsafe extern "C" fn dav1d_open() {}\ntype F = unsafe extern "C" fn(u8);\nextern "C" {\n    fn f();\n}\n'
        out = generate.unwind_abi(source)
        self.assertEqual(out.count('extern "C-unwind"'), 3)
        self.assertNotRegex(out, r'extern "C"(?!-)')

    def test_an_unwinding_abi_is_left_alone(self):
        source = 'extern "C-unwind" fn f() {}\nextern "system" fn g() {}\n'
        self.assertEqual(generate.unwind_abi(source), source)


class Manifest(unittest.TestCase):
    def test_renames_the_package_and_keeps_the_library_name(self):
        out = generate.manifest(UPSTREAM_MANIFEST, PINS)
        self.assertIn(f'name = "{PINS["package"]}"\nversion = "{PINS["version"]}"\n', out)
        self.assertIn('[lib]\nname = "rav1d"\n', out)
        self.assertIn(f'repository = "{PINS["repository"]}"', out)
        self.assertIn('license = "BSD-2-Clause"', out)
        self.assertNotIn("memorysafety", out)

    def test_an_unexpected_upstream_manifest_is_rejected(self):
        with self.assertRaisesRegex(generate.GenerationError, "expected exactly one"):
            generate.manifest(UPSTREAM_MANIFEST.replace('version = "1.1.0"', 'version = "1.1.1"'), PINS)

    def test_a_renamed_library_is_rejected(self):
        with self.assertRaisesRegex(generate.GenerationError, "library must stay named rav1d"):
            generate.manifest(UPSTREAM_MANIFEST.replace('[lib]\nname = "rav1d"', '[lib]\nname = "x"'), PINS)


class Archive(unittest.TestCase):
    def test_a_tampered_archive_is_rejected(self):
        with self.assertRaisesRegex(generate.GenerationError, "sha256"):
            generate.verified(b"not the crate", PINS)


class Tree(unittest.TestCase):
    def test_tree_hash_covers_paths_and_contents_but_not_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("a")
            first = generate.tree_sha256(root)
            (root / "target").mkdir()
            (root / "target/x").write_text("build output")
            self.assertEqual(generate.tree_sha256(root), first)
            (root / "src/lib.rs").write_text("b")
            self.assertNotEqual(generate.tree_sha256(root), first)


if __name__ == "__main__":
    unittest.main()
