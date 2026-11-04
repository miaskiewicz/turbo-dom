"""Self-test of scripts/check-glibc-ceiling.sh (the release glibc guard).

  python3 -m unittest discover -s scripts -p 'test_*.py'

A stub `readelf` on PATH stands in for binutils, so the test runs on any host. The stub
prints the fixture file's own contents as the `readelf -W --dyn-syms -V` output, and
fails (like readelf on a non-ELF file) when the fixture holds the line `READELF-FAIL`.
A stub `objdump` that always fails with "file format not recognized" sits beside it: that
is what host binutils `objdump -T` does on a foreign-arch ELF (the arm64 addon on the x64
publish runner), so the guard must never depend on objdump.
"""
import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check-glibc-ceiling.sh"

STUB = """#!/bin/sh
[ "$1 $2 $3" = "-W --dyn-syms -V" ] || exit 64
if grep -q '^READELF-FAIL$' "$4" 2>/dev/null; then
  echo "readelf: Error: $4: Not an ELF file" >&2
  exit 1
fi
cat "$4"
"""
OBJDUMP_STUB = """#!/bin/sh
echo "objdump: $2: file format not recognized" >&2
exit 1
"""

OK_SYMBOLS = """
Symbol table '.dynsym' contains 4 entries:
   Num:    Value          Size Type    Bind   Vis      Ndx Name
     1: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND malloc@GLIBC_2.2.5 (2)
     2: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND __libc_start_main@GLIBC_2.34 (3)
     3: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND memcpy@GLIBC_2.35 (4)
"""
OVER_SYMBOLS = OK_SYMBOLS + (
    "     4: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND __isoc23_strtol@GLIBC_2.39 (5)\n"
)
NO_GLIBC_SYMBOLS = """
     1: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND napi_create_object
"""
# `readelf -W --dyn-syms -V` of an aarch64 addon (as on the x64 publish runner): the
# symbols plus the .gnu.version_r needs, where the highest version may appear only there.
AARCH64_SYMBOLS = """
Symbol table '.dynsym' contains 3 entries:
   Num:    Value          Size Type    Bind   Vis      Ndx Name
     1: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND malloc@GLIBC_2.17 (2)
     2: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND napi_create_object

Version needs section '.gnu.version_r' contains 1 entry:
 Addr: 0x0000000000000a10  Offset: 0x000a10  Link: 5 (.dynstr)
  000000: Version: 1  File: libc.so.6  Cnt: 2
  0x0010:   Name: GLIBC_2.17  Flags: none  Version: 2
  0x0020:   Name: GLIBC_2.34  Flags: none  Version: 3
"""


class GlibcCeilingTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(self.enterContext(tempfile.TemporaryDirectory()))
        stub_dir = self.tmp / "bin"
        stub_dir.mkdir()
        for name, body in (("readelf", STUB), ("objdump", OBJDUMP_STUB)):
            stub = stub_dir / name
            stub.write_text(body)
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

    def test_a_readelf_failure_fails(self):
        result = self.run_guard(self.node("broken.node", "READELF-FAIL\n"))
        self.assert_fails(result, "readelf failed")

    def test_a_foreign_arch_binary_is_read_without_objdump(self):
        # objdump (stubbed to fail like host binutils on aarch64) must not be consulted.
        result = self.run_guard(self.node("a.linux-arm64-gnu.node", AARCH64_SYMBOLS))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("highest required GLIBC_2.34", result.stdout)

    def test_no_glibc_symbols_fails(self):
        # Every guarded file is a dynamically linked linux-gnu addon, which always needs
        # some GLIBC_ symbol: none at all means readelf read the wrong thing.
        result = self.run_guard(self.node("a.node", NO_GLIBC_SYMBOLS))
        self.assert_fails(result, "no GLIBC_ symbol versions")


if __name__ == "__main__":
    unittest.main()
