"""Self-test of scripts/check-glibc-ceiling.sh (the release glibc guard).

  python3 -m unittest discover -s scripts -p 'test_*.py'

A stub `objdump` on PATH stands in for binutils, so the test runs on any host. The stub
prints the fixture file's own contents as the `objdump -T` dynamic symbol table, and
fails (like objdump on a non-ELF file) when the fixture holds the line `OBJDUMP-FAIL`.
"""
import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check-glibc-ceiling.sh"

STUB = """#!/bin/sh
[ "$1" = "-T" ] || exit 64
if grep -q '^OBJDUMP-FAIL$' "$2" 2>/dev/null; then
  echo "objdump: $2: file format not recognized" >&2
  exit 1
fi
cat "$2"
"""

OK_SYMBOLS = """
0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.2.5) malloc
0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.34) __libc_start_main
0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.35) memcpy
"""
OVER_SYMBOLS = OK_SYMBOLS + (
    "0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.39) __isoc23_strtol\n"
)
NO_GLIBC_SYMBOLS = """
0000000000000000      DF *UND*  0000000000000000  napi_create_object
"""


class GlibcCeilingTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(self.enterContext(tempfile.TemporaryDirectory()))
        stub_dir = self.tmp / "bin"
        stub_dir.mkdir()
        stub = stub_dir / "objdump"
        stub.write_text(STUB)
        stub.chmod(stub.stat().st_mode | stat.S_IXUSR)
        self.env = {**os.environ, "PATH": f"{stub_dir}{os.pathsep}{os.environ['PATH']}"}

    def node(self, name, contents):
        path = self.tmp / name
        path.write_text(contents)
        return str(path)

    def run_guard(self, *args):
        return subprocess.run(
            ["bash", str(SCRIPT), *args],
            env=self.env,
            cwd=self.tmp,
            capture_output=True,
            text=True,
            check=False,
        )

    def assert_fails(self, result, needle):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(needle, result.stdout + result.stderr)

    def test_ok_at_the_ceiling(self):
        result = self.run_guard(self.node("a.linux-x64-gnu.node", OK_SYMBOLS))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("highest required GLIBC_2.35", result.stdout)

    def test_symbols_over_the_ceiling_fail(self):
        result = self.run_guard(
            self.node("ok.node", OK_SYMBOLS), self.node("over.node", OVER_SYMBOLS)
        )
        self.assert_fails(result, "requires GLIBC_2.39 (> GLIBC_2.35)")

    def test_a_lower_ceiling_is_honoured(self):
        self.env["GLIBC_CEILING"] = "2.34"
        result = self.run_guard(self.node("a.node", OK_SYMBOLS))
        self.assert_fails(result, "requires GLIBC_2.35 (> GLIBC_2.34)")

    def test_a_missing_file_fails(self):
        result = self.run_guard(self.node("ok.node", OK_SYMBOLS), "missing.node")
        self.assert_fails(result, "missing.node")
        self.assertIn("no such file", result.stdout + result.stderr)

    def test_a_glob_that_matches_nothing_fails(self):
        # bash passes an unmatched glob through literally (no nullglob in CI).
        result = self.run_guard("*linux-*-gnu.node")
        self.assert_fails(result, "*linux-*-gnu.node")

    def test_no_arguments_is_a_usage_error(self):
        result = self.run_guard()
        self.assertEqual(result.returncode, 2)
        self.assertIn("usage", result.stderr)

    def test_an_objdump_failure_fails(self):
        result = self.run_guard(self.node("broken.node", "OBJDUMP-FAIL\n"))
        self.assert_fails(result, "objdump -T failed")

    def test_no_glibc_symbols_fails(self):
        # Every guarded file is a dynamically linked linux-gnu addon, which always needs
        # some GLIBC_ symbol: none at all means objdump read the wrong thing.
        result = self.run_guard(self.node("a.node", NO_GLIBC_SYMBOLS))
        self.assert_fails(result, "no GLIBC_ symbol versions")


if __name__ == "__main__":
    unittest.main()
