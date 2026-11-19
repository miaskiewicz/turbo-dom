#!/usr/bin/env python3
"""Generate the publishable turbo-dom-rav1d crate from upstream rav1d 1.1.0.

turbo-dom-rav1d is rav1d 1.1.0 (BSD-2-Clause, the ISRG Prossimo Rust port of dav1d) with
two changes, so a decoder panic is a broken image instead of a process abort
(miaskiewicz/turbo-dom#535):

1. ``patches/on-error-frame-hdr.patch``: the upstream fix for memorysafety/rav1d#1497
   (commit 6c6f46378, on rav1d ``main`` but in no published release): ``on_error`` no
   longer unwraps a frame header that ``rav1d_decode_frame_exit`` already took, so any
   decode error with one frame context returns an error instead of panicking.
2. every ``extern "C"`` ABI becomes ``extern "C-unwind"`` (``unwind_abi()``): a panic
   inside rav1d's C API -- or inside a DSP routine it calls through a C function
   pointer -- then unwinds to the caller, which can ``catch_unwind`` it. Behind
   ``extern "C"`` it cannot unwind and aborts the process (memorysafety/rav1d#1521).

Plus the manifest transform in ``manifest()`` (package name, description, repository)
and the removal of upstream packaging records and its nightly ``rust-toolchain.toml``.
The upstream archive is checksum-pinned in ``pins.json``; nothing upstream is committed
to this repository.

  generate.py [--download] --write [--output DIR]  write the crate (default crate/)
  generate.py [--download] --check                 regenerate and compare with pins
  generate.py --write --update-pins                accept a reviewed transform change
"""

import argparse
import hashlib
import io
import json
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
PINS_PATH = CRATE / "pins.json"
DEFAULT_OUTPUT = CRATE / "crate"
DOWNLOAD_DIR = CRATE / ".download"
CARGO_CACHE = Path.home() / ".cargo" / "registry" / "cache"
PATCHES = ["on-error-frame-hdr.patch"]
# Upstream packaging records (Cargo regenerates them for the renamed package) and
# upstream's nightly toolchain pin, which would make rustup switch channels inside the
# generated directory.
REMOVED = ["Cargo.toml.orig", "Cargo.lock", ".cargo_vcs_info.json", "rust-toolchain.toml"]
C_ABI = re.compile(r'extern "C"(?!-)')


class GenerationError(Exception):
    pass


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def load_pins():
    return json.loads(PINS_PATH.read_text())


def archive_bytes(pins, download):
    """The pinned upstream .crate: Cargo's registry cache, else the download dir,
    else (with ``download``) static.crates.io."""
    name = pins["input"]["archive"]
    candidates = sorted(CARGO_CACHE.glob(f"*/{name}")) + [DOWNLOAD_DIR / name]
    for path in candidates:
        if path.is_file():
            return verified(path.read_bytes(), pins)
    if not download:
        raise GenerationError(f"{name} is not in Cargo's cache; pass --download")
    with urllib.request.urlopen(pins["input"]["url"], timeout=60) as response:
        data = response.read()
    verified(data, pins)
    DOWNLOAD_DIR.mkdir(exist_ok=True)
    (DOWNLOAD_DIR / name).write_bytes(data)
    return data


def verified(data, pins):
    actual = sha256(data)
    if actual != pins["input"]["archive_sha256"]:
        raise GenerationError(f"upstream archive sha256 {actual} != pinned {pins['input']['archive_sha256']}")
    return data


def extract(data, pins, into):
    prefix = f"{pins['upstream']['package']}-{pins['upstream']['version']}/"
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive.getmembers():
            if not member.name.startswith(prefix) or ".." in Path(member.name).parts:
                raise GenerationError(f"unexpected archive entry {member.name}")
            if not (member.isfile() or member.isdir()):
                raise GenerationError(f"unexpected archive entry type {member.name}")
        archive.extractall(into, filter="data")
    return Path(into) / prefix.rstrip("/")


def unwind_abi(text):
    """Every ``extern "C"`` ABI as ``extern "C-unwind"``; already-unwinding ABIs stay."""
    return C_ABI.sub('extern "C-unwind"', text)


def manifest(text, pins):
    """Rename the normalized upstream manifest to the fork's package."""
    replacements = [
        # The [package] name (the [lib] keeps `name = "rav1d"`, so `rav1d::` paths hold).
        ('rust-version = "1.79"\nname = "rav1d"\nversion = "1.1.0"\n',
         f'rust-version = "1.79"\nname = "{pins["package"]}"\nversion = "{pins["version"]}"\n'),
        ('description = "Rust port of the dav1d AV1 decoder"\n', f'description = "{pins["description"]}"\n'),
        ('repository = "https://github.com/memorysafety/rav1d"\n', f'repository = "{pins["repository"]}"\n'),
    ]
    for old, new in replacements:
        if text.count(old) != 1:
            raise GenerationError(f"manifest: expected exactly one {old.strip()!r}")
        text = text.replace(old, new)
    if '[lib]\nname = "rav1d"\n' not in text:
        raise GenerationError("manifest: the library must stay named rav1d")
    return text


def transform(root, pins):
    for patch in PATCHES:
        subprocess.run(
            ["patch", "-p1", "--forward", "--no-backup-if-mismatch", "--quiet", "-i", str(CRATE / "patches" / patch)],
            cwd=root, check=True,
        )
    rewritten = 0
    for path in sorted(root.rglob("*.rs")):
        text = path.read_text()
        new = unwind_abi(text)
        if new != text:
            rewritten += len(C_ABI.findall(text))
            path.write_text(new)
    if rewritten != pins["transform"]["c_abi_rewrites"]:
        raise GenerationError(f'{rewritten} extern "C" rewrites, pinned {pins["transform"]["c_abi_rewrites"]}')
    for name in REMOVED:
        (root / name).unlink(missing_ok=True)
    (root / "Cargo.toml").write_text(manifest((root / "Cargo.toml").read_text(), pins))


def tree_sha256(root):
    digest = hashlib.sha256()
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        relative = path.relative_to(root).as_posix()
        if relative.startswith("target/"):
            continue
        digest.update(relative.encode() + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def generate(download):
    pins = load_pins()
    work = Path(tempfile.mkdtemp(prefix="turbo-dom-rav1d-"))
    root = extract(archive_bytes(pins, download), pins, work)
    transform(root, pins)
    return pins, work, root


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--download", action="store_true")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--update-pins", action="store_true")
    args = parser.parse_args(argv)
    pins, work, root = generate(args.download)
    try:
        tree = tree_sha256(root)
        if args.update_pins:
            pins["generated"]["tree_sha256"] = tree
            PINS_PATH.write_text(json.dumps(pins, indent=2) + "\n")
        elif tree != pins["generated"]["tree_sha256"]:
            raise GenerationError(f"generated tree sha256 {tree} != pinned {pins['generated']['tree_sha256']}")
        if args.write:
            shutil.rmtree(args.output, ignore_errors=True)
            shutil.copytree(root, args.output)
            print(f"wrote {pins['package']} {pins['version']} to {args.output} (tree {tree[:12]})")
        else:
            print(f"{pins['package']} {pins['version']} reproduces (tree {tree[:12]})")
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    try:
        main()
    except GenerationError as error:
        sys.exit(f"generate.py: {error}")
