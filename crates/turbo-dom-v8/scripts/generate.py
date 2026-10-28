#!/usr/bin/env python3
"""Generate the publishable turbo-dom-v8 crate from upstream rusty_v8 150.4.0.

Inputs (all in this repository except the checksum-pinned upstream archive):
1. upstream ``v8-150.4.0.crate`` (static.crates.io, sha256 in ``pins.json``), taken
   from ``--archive``, else Cargo's registry cache, else (``--download``) fetched into
   the gitignored ``crates/turbo-dom-v8/.download/``. Only its Rust subset is used:
   root files plus ``pins.json`` ``included_directories`` (no C++ trees).
2. ``patches/rusty-v8-150.patch`` (moved from turbo-surf's
   rust/vendor/rusty-v8-150.patch): src/function.rs interceptor ShouldThrowOnError fix,
   src/isolate.rs weak teardown fix, build.rs source-build guard, MIT LICENSE.
3. the turbo-dom deltas: ``patches/{build-rs,fork-marker,v8-teardown-tests,
   lib-test-init}.patch``, the manifest transform in ``manifest()``, the reserved-file
   removal and our regression tests (``tests/``) and ``rustfmt.toml``.
Every stage is checked against ``pins.json`` before anything is written.

  generate.py [--download] --write [--output DIR]  write the crate (default crate/)
  generate.py [--download] --check [--output DIR]  regenerate and compare with pins
                                                    (and with the output dir if present)
  generate.py --verify DIR                          hash-check a generated crate only
  generate.py ... --compare DIR                     also require the patched upstream tree
                                                    (stage 2) to equal DIR byte-for-byte
  generate.py --write --update-pins                 accept reviewed patch/test changes

Nothing here builds V8 or modifies Cargo's cache.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.request
from pathlib import Path, PurePosixPath

CRATE = Path(__file__).resolve().parents[1]
PINS_PATH = CRATE / "pins.json"
DEFAULT_OUTPUT = CRATE / "crate"
DOWNLOAD_DIR = CRATE / ".download"
ARCHIVE_LIMIT = 64 * 1024 * 1024
UPSTREAM_PATCH = "rusty-v8-150.patch"
PATCHES = [
    UPSTREAM_PATCH,             # function.rs + isolate.rs fixes, build.rs guard, LICENSE
    "build-rs.patch",           # UPSTREAM_V8_VERSION at both download sites + guard + unit test
    "fork-marker.patch",        # pub const FORK_SHOULD_THROW_FIX + crate-level lint allows
    "v8-teardown-tests.patch",  # turbo-surf release-deps teardown tests (tests/test_api.rs)
    "lib-test-init.patch",      # upstream lib tests both initialized V8; share the Once
]
# Our regression tests, copied into the generated crate as explicit [[test]]
# targets (upstream sets autotests = false).
OWN_TESTS = ["interceptor_should_throw", "release_v8_teardown", "weak_teardown", "fork_marker"]
# Upstream packaging records and upstream-only developer files; Cargo regenerates the
# first three for the renamed package and rust-toolchain.toml would pin rustup to
# upstream's channel inside the generated directory.
REMOVED = ["Cargo.toml.orig", "Cargo.lock", ".cargo_vcs_info.json", "rust-toolchain.toml"]
# Upstream targets that need third_party/icu/common/icudtl.dat (not in the
# prebuilt distribution) or trybuild, and the dev-dependencies only they use.
STRIPPED_TESTS = ["test_api", "test_cppgc", "test_ui"]
STRIPPED_DEV_DEPENDENCIES = ["align-data", "rustversion", "trybuild"]


class GenerationError(Exception):
    pass


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def files(directory):
    """Map of POSIX relative path -> bytes for every regular file under directory."""
    result = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise GenerationError(f"unexpected symlink: {path}")
        if path.is_file():
            result[path.relative_to(directory).as_posix()] = path.read_bytes()
    return result


def tree_digest(tree):
    lines = "".join(f"{path}\0{sha256(data)}\n" for path, data in sorted(tree.items()))
    return sha256(lines.encode())


def load_pins(path=PINS_PATH):
    return json.loads(path.read_text())


def checked_archive(path, pins):
    if path.is_symlink() or not path.is_file() or sha256(path.read_bytes()) != pins["input"]["archive_sha256"]:
        raise GenerationError(f"missing or modified upstream archive: {path}")
    return path


def find_archive(pins, archive=None, cargo_home=None, download=False, urlopen=urllib.request.urlopen,
                 download_dir=DOWNLOAD_DIR):
    """Resolve the checksum-pinned upstream crate archive (never modifies Cargo's cache)."""
    name = pins["input"]["archive"]
    if archive is not None:
        return checked_archive(Path(archive), pins)
    home = Path(cargo_home or os.environ.get("CARGO_HOME") or Path.home() / ".cargo")
    for cached in sorted(home.glob(f"registry/cache/*/{name}")):
        return checked_archive(cached, pins)
    downloaded = download_dir / name
    if downloaded.exists() or downloaded.is_symlink():
        return checked_archive(downloaded, pins)
    if not download:
        raise GenerationError(f"{name} is not in Cargo's cache; pass --archive PATH or --download")
    request = urllib.request.Request(pins["input"]["url"], headers={"User-Agent": "turbo-dom-v8-generate"})
    with urlopen(request, timeout=120) as response:
        payload = response.read(ARCHIVE_LIMIT + 1)
    if len(payload) > ARCHIVE_LIMIT:
        raise GenerationError("upstream archive exceeds the size limit")
    if sha256(payload) != pins["input"]["archive_sha256"]:
        raise GenerationError("downloaded upstream archive checksum mismatch")
    download_dir.mkdir(parents=True, exist_ok=True)
    partial = downloaded.with_suffix(".partial")
    partial.write_bytes(payload)
    partial.replace(downloaded)
    return checked_archive(downloaded, pins)


def extract(archive, pins):
    """Rust subset of the upstream archive: root files plus the included directories."""
    prefix = f"{pins['upstream']['package']}-{pins['upstream']['version']}"
    included = set(pins["input"]["included_directories"])
    tree = {}
    with tarfile.open(archive, mode="r:gz") as package:
        for entry in package:
            if entry.isdir():
                continue
            if not entry.isfile():
                raise GenerationError(f"unexpected archive entry type: {entry.name}")
            path = PurePosixPath(entry.name)
            if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != prefix:
                raise GenerationError(f"unsafe archive entry: {entry.name}")
            relative = PurePosixPath(*path.parts[1:])
            if len(relative.parts) > 1 and relative.parts[0] not in included:
                continue
            tree[str(relative)] = package.extractfile(entry).read()
    return tree


def check(tree, expected, label):
    if len(tree) != expected[f"{label}_files"] or tree_digest(tree) != expected[f"{label}_tree_sha256"]:
        raise GenerationError(f"{label} upstream tree differs from pins.json")


def write_tree(tree, destination):
    for relative, data in tree.items():
        path = destination / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def replace_field(body, key, value):
    line = f"{key} = {json.dumps(value)}"
    pattern = rf"(?m)^{re.escape(key)} = .*$"
    if not re.search(pattern, body):
        raise GenerationError(f"upstream [package] lost {key}")
    return re.sub(pattern, lambda _: line, body, count=1)


def manifest(source, pins):
    """Rename upstream v8 to turbo-dom-v8 and reshape its targets; keep [lib] name v8."""
    upstream = pins["upstream"]
    parsed = tomllib.loads(source)
    package = parsed["package"]
    if (package["name"], package["version"]) != (upstream["package"], upstream["version"]):
        raise GenerationError("unexpected upstream manifest identity")
    if parsed["lib"] != {"name": "v8", "path": "src/lib.rs"} or "links" in package:
        raise GenerationError("upstream library/link contract changed; review the transform")
    tests = {test["name"] for test in parsed["test"]}
    if not set(STRIPPED_TESTS) <= tests or set(OWN_TESTS) & tests:
        raise GenerationError("upstream test targets changed; review the strip list")
    sections = re.split(r"(?m)^(\[\[?[^\n]+\]\]?\n)", source)
    if not sections[0].startswith("# THIS FILE IS AUTOMATICALLY GENERATED BY CARGO\n"):
        raise GenerationError("upstream manifest is not Cargo's normalized form")
    output = [
        "# Generated by crates/turbo-dom-v8/scripts/generate.py (miaskiewicz/turbo-dom)\n"
        f"# from the normalized manifest of upstream v8 {upstream['version']}: renamed to\n"
        f"# {pins['package']}, links rusty_v8, LICENSE kept, icudtl.dat/trybuild targets\n"
        "# stripped, turbo-dom regression tests added. Do not edit; edit the generator.\n\n"
    ]
    for index in range(1, len(sections), 2):
        header, body = sections[index], sections[index + 1]
        if header == "[package]\n":
            if '    "LICENSE*",\n' not in body:
                raise GenerationError("upstream LICENSE exclusion changed")
            # Keep the retained MIT license in the published archive.
            body = body.replace('    "LICENSE*",\n', "")
            body = replace_field(body, "name", pins["package"])
            body = replace_field(body, "version", pins["version"])
            body = replace_field(body, "repository", pins["repository"])
            body = replace_field(body, "description", pins["description"])
            body = body.replace('build = "build.rs"\n', 'build = "build.rs"\nlinks = "rusty_v8"\n', 1)
            body = body.replace('edition = "2024"\n', f'edition = "2024"\nrust-version = "{pins["rust_version"]}"\n', 1)
            if 'links = "rusty_v8"' not in body or "rust-version" not in body:
                raise GenerationError("upstream [package] layout changed")
        elif header == "[lib]\n":
            # Upstream doc examples do not compile (e.g. src/scope.rs imports Object
            # twice); upstream never runs them.
            body = body.rstrip("\n") + "\ndoctest = false\n\n"
        elif header == "[[test]]\n":
            if tomllib.loads(body)["name"] in STRIPPED_TESTS:
                continue
        elif header.startswith("[dev-dependencies."):
            if header[len("[dev-dependencies."):-2] in STRIPPED_DEV_DEPENDENCIES:
                continue
        output += [header, body]
    result = "".join(output).rstrip("\n") + "\n"
    for name in OWN_TESTS:
        result += f'\n[[test]]\nname = "{name}"\npath = "tests/{name}.rs"\n'
    result += (
        "\n[package.metadata.turbo-dom-v8]\n"
        f'upstream-package = "{upstream["package"]}"\n'
        f'upstream-version = "{upstream["version"]}"\n'
        f'upstream-revision = "{upstream["revision"]}"\n'
        f'upstream-repository = "{upstream["repository"]}"\n'
        f'scope = "{pins["scope"]}"\n'
    )
    return result


def apply_patches(target, pins, names):
    for name in names:
        patch = CRATE / "patches" / name
        if sha256(patch.read_bytes()) != pins["patches"][name]:
            raise GenerationError(f"patch changed: {name} (rerun with --update-pins after review)")
        subprocess.run(["patch", "--batch", "--forward", "-p1", "--fuzz=0", "--no-backup-if-mismatch",
                        "-i", str(patch)], cwd=target, check=True, stdout=subprocess.DEVNULL)


def generate(archive, destination, pins, update_pins=False, compare=None):
    """Write the generated crate into the empty directory destination; return its tree."""
    expected = pins["input"]
    tree = extract(checked_archive(archive, pins), pins)
    check(tree, expected, "extracted")
    for relative, key in [("src/function.rs", "original_function_sha256"),
                          ("src/isolate.rs", "original_isolate_sha256")]:
        if sha256(tree[relative]) != expected[key]:
            raise GenerationError(f"upstream {relative} does not match the reviewed original")
    write_tree(tree, destination)
    if update_pins:
        pins["patches"] = {name: sha256((CRATE / "patches" / name).read_bytes()) for name in PATCHES}
    apply_patches(destination, pins, [UPSTREAM_PATCH])
    patched = files(destination)
    for relative, key in [("src/function.rs", "patched_function_sha256"),
                          ("src/isolate.rs", "patched_isolate_sha256"),
                          ("LICENSE", "license_sha256")]:
        if sha256(patched[relative]) != expected[key]:
            raise GenerationError(f"patched {relative} does not match the reviewed fix")
    check(patched, expected, "patched")
    if compare is not None:
        other = files(Path(compare))
        different = sorted(path for path in patched.keys() | other.keys() if patched.get(path) != other.get(path))
        if different:
            raise GenerationError(f"patched upstream tree differs from {compare}: " + ", ".join(different))
    for name in REMOVED:
        (destination / name).unlink()
    if update_pins:
        pins["patches"] = {name: sha256((CRATE / "patches" / name).read_bytes()) for name in PATCHES}
    apply_patches(destination, pins, PATCHES[1:])
    leftovers = [p for p in destination.rglob("*") if p.suffix in {".orig", ".rej"}]
    if leftovers:
        raise GenerationError(f"patch left {leftovers}")
    cargo = destination / "Cargo.toml"
    # Bytes, not write_text: Windows text mode would turn every "\n" into "\r\n".
    cargo.write_bytes(manifest(cargo.read_text(), pins).encode())
    for name in OWN_TESTS:
        shutil.copyfile(CRATE / "tests" / f"{name}.rs", destination / "tests" / f"{name}.rs")
    shutil.copyfile(CRATE / "rustfmt.toml", destination / "rustfmt.toml")
    generated = files(destination)
    build = generated["build.rs"].decode()
    if 'env::var("CARGO_PKG_VERSION")' in build or \
            f'const UPSTREAM_V8_VERSION: &str = "{pins["upstream"]["version"]}";' not in build:
        raise GenerationError("build.rs must download the pinned upstream SDK version")
    if "pub const FORK_SHOULD_THROW_FIX: bool = true;" not in generated["src/lib.rs"].decode():
        raise GenerationError("fork marker missing")
    if sha256(generated["tests/test_api.rs"]) != pins["generated"]["teardown_tests_sha256"]:
        raise GenerationError("release teardown tests changed")
    record = {"files": len(generated), "tree_sha256": tree_digest(generated),
              "teardown_tests_sha256": pins["generated"]["teardown_tests_sha256"],
              "key_files": {name: sha256(generated[name]) for name in pins["generated"]["key_files"]}}
    if update_pins:
        pins["generated"] = record
    elif record != pins["generated"]:
        raise GenerationError("generated crate differs from pins.json (review, then --update-pins)")
    return generated


def verify_output(output, pins):
    actual = files(output)
    # Cargo writes these next to a standalone package; they are not crate sources.
    actual = {path: data for path, data in actual.items()
              if path != "Cargo.lock" and not path.startswith("target/")}
    expected = pins["generated"]
    if len(actual) != expected["files"] or tree_digest(actual) != expected["tree_sha256"]:
        raise GenerationError(f"{output} is not the pinned generated turbo-dom-v8 crate")
    for name, digest in expected["key_files"].items():
        if sha256(actual[name]) != digest:
            raise GenerationError(f"{output}/{name} hash differs")
    return actual


def replace_contents(output, generated):
    output.mkdir(parents=True, exist_ok=True)
    for path in output.iterdir():
        # Keep build products of a previous generation (standalone package).
        if path.name in {"target", "Cargo.lock"}:
            continue
        shutil.rmtree(path) if path.is_dir() else path.unlink()
    for relative, data in generated.items():
        path = output / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--archive", type=Path, help="upstream v8-150.4.0.crate (default: Cargo's cache)")
    parser.add_argument("--download", action="store_true",
                        help="fetch the pinned archive from static.crates.io if it is not cached")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--compare", type=Path, metavar="DIR",
                        help="require upstream + rusty-v8-150.patch to equal DIR (e.g. a turbo-surf vendor tree)")
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--write", action="store_true")
    modes.add_argument("--check", action="store_true")
    modes.add_argument("--verify", type=Path, metavar="DIR")
    parser.add_argument("--update-pins", action="store_true")
    args = parser.parse_args(argv)
    pins = load_pins()
    if args.update_pins and not args.write:
        parser.error("--update-pins requires --write")
    try:
        if args.verify:
            verify_output(args.verify, pins)
            print(f"Verified generated turbo-dom-v8 {pins['version']} at {args.verify}")
            return 0
        archive = find_archive(pins, args.archive, download=args.download)
        with tempfile.TemporaryDirectory(prefix="turbo-dom-v8-") as scratch:
            generated = generate(archive, Path(scratch), pins, args.update_pins, args.compare)
            compared = ""
            if args.check:
                if args.output.exists():
                    if verify_output(args.output, pins) != generated:
                        raise GenerationError("output differs from the regenerated crate")
                    compared = f", output {args.output} identical"
                else:
                    compared = f", no output at {args.output}"
            else:
                replace_contents(args.output, generated)
                compared = f", written to {args.output}"
                if args.update_pins:
                    PINS_PATH.write_bytes((json.dumps(pins, indent=2) + "\n").encode())
    except GenerationError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    action = "Checked" if args.check else "Generated"
    print(f"{action} turbo-dom-v8 {pins['version']} from {archive.name} (upstream v8 {pins['upstream']['version']}): "
          f"{pins['generated']['files']} files, tree {pins['generated']['tree_sha256'][:12]}"
          + (f", stage 2 identical to {args.compare}" if args.compare else "") + compared)
    return 0


if __name__ == "__main__":
    sys.exit(main())
