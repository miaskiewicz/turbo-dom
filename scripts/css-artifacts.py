#!/usr/bin/env python3
"""Build/check the lazy CSS payload without replacing the parser's artifacts.

Build commands use the caller's Cargo toolchain/cache/jobs configuration. `plan`
only prints commands; it never invokes a compiler. `check` validates the complete
release payload, while `bundle` copies only an exact reviewed native matrix.
"""
import argparse
import hashlib
import json
import os
import shutil
import re
import tempfile
import tomllib
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
TARGETS = {
    "x86_64-unknown-linux-gnu": "linux-x64-gnu",
    "x86_64-unknown-linux-musl": "linux-x64-musl",
    "aarch64-unknown-linux-gnu": "linux-arm64-gnu",
    "aarch64-apple-darwin": "darwin-arm64",
    "x86_64-pc-windows-msvc": "win32-x64-msvc",
}
WASM_PACKAGES = {"pkg": "turbo_dom_parser", "pkg-web": "turbo_dom_parser",
                 "pkg-css": "turbo_dom_css", "pkg-css-web": "turbo_dom_css"}
WASM_SUFFIXES = (".js", ".d.ts", "_bg.wasm", "_bg.wasm.d.ts")


def native_plan(target, target_dir):
    if target not in TARGETS:
        raise ValueError(f"unsupported CSS native target: {target}")
    library = "libturbo_dom_parser.so"
    if "apple" in target: library = "libturbo_dom_parser.dylib"
    if "windows" in target: library = "turbo_dom_parser.dll"
    command = ["cargo", "build", "--locked", "--release", "--no-default-features",
               "--features", "napi-bind,css-backend", "--target", target]
    return command, target_dir / target / "release" / library, Path("native-css") / f"turbo-dom-css.{TARGETS[target]}.node"


def wasm_plan(target):
    directories = {"nodejs": "pkg-css", "web": "pkg-css-web"}
    if target not in directories:
        raise ValueError(f"unsupported CSS WASM target: {target}")
    return ["wasm-pack", "build", "--target", target, "--out-dir", directories[target],
            "--out-name", "turbo_dom_css", "--no-default-features", "--features", "wasm-bind,css-backend"]


def normalize_wasm_manifest(target, root):
    directory = "pkg-css" if target == "nodejs" else "pkg-css-web"
    path = root / directory / "package.json"
    package = json.loads(path.read_text())
    package["private"] = True
    package["type"] = "commonjs" if target == "nodejs" else "module"
    path.write_text(json.dumps(package, indent=2) + "\n")


def cargo_target_directory(root, environment, runner=subprocess.check_output):
    metadata = json.loads(runner(["cargo", "metadata", "--offline", "--locked", "--no-deps", "--format-version", "1"],
                                 cwd=root, env=environment, text=True))
    value = metadata.get("target_directory")
    if not isinstance(value, str) or not Path(value).is_absolute():
        raise ValueError("locked Cargo metadata has no absolute target_directory")
    return Path(value)


def cached_tools(environment):
    bindgen = environment.get("TURBO_DOM_WASM_BINDGEN")
    optimizer = environment.get("TURBO_DOM_WASM_OPT")
    if bool(bindgen) != bool(optimizer):
        raise ValueError("set both TURBO_DOM_WASM_BINDGEN and TURBO_DOM_WASM_OPT for cached offline tools")
    return (bindgen, optimizer) if bindgen else None


def verify_cached_tools(root, tools, runner=subprocess.check_output):
    packages = tomllib.loads((root / "Cargo.lock").read_text())["package"]
    versions = [package["version"] for package in packages if package["name"] == "wasm-bindgen"]
    if len(versions) != 1:
        raise ValueError("cached CSS build requires exactly one locked wasm-bindgen version")
    actual = runner([tools[0], "--version"], text=True).strip()
    if actual != f"wasm-bindgen {versions[0]}":
        raise ValueError(f"{actual} does not match locked wasm-bindgen {versions[0]}")
    optimizer = runner([tools[1], "--version"], text=True).strip()
    if not re.search(r"\bversion 117\b", optimizer):
        raise ValueError(f"{optimizer} does not match reviewed wasm-opt117")


def cached_wasm_plan(target, target_dir, stage, bindgen, optimizer):
    wasm_plan(target)  # Share the target admission with the wasm-pack route.
    source = target_dir / "wasm32-unknown-unknown/release/turbo_dom_parser.wasm"
    output = stage / "turbo_dom_css_bg.wasm"
    optimized = stage / "turbo_dom_css_optimized.wasm"
    return [
        ["cargo", "build", "--offline", "--locked", "--release", "--no-default-features", "--features",
         "wasm-bind,css-backend", "--target", "wasm32-unknown-unknown"],
        [bindgen, str(source), "--target", target, "--out-dir", str(stage), "--out-name", "turbo_dom_css"],
        [optimizer, "-Oz", str(output), "-o", str(optimized)],
    ]


def write_cached_manifest(root, stage, target):
    version = json.loads((root / "package.json").read_text())["version"]
    package = {"name": "turbo-dom-css", "version": version, "private": True,
               "type": "commonjs" if target == "nodejs" else "module", "types": "turbo_dom_css.d.ts",
               "files": [f"turbo_dom_css{suffix}" for suffix in WASM_SUFFIXES] + ["snippets"]}
    package["main" if target == "nodejs" else "module"] = "turbo_dom_css.js"
    (stage / "package.json").write_text(json.dumps(package, indent=2) + "\n")


def replace_wasm_package(stage, destination):
    backup = stage.parent / "previous"
    if destination.exists(): destination.rename(backup)
    try:
        stage.rename(destination)
    except BaseException:
        if backup.exists(): backup.rename(destination)
        raise
    if backup.is_symlink(): backup.unlink()
    elif backup.exists(): shutil.rmtree(backup)


def build_cached_wasm(root, target, tools, environment):
    verify_cached_tools(root, tools)
    command = cached_wasm_plan(target, Path("target"), Path("stage"), *tools)[0]
    subprocess.run(command, cwd=root, env=environment, check=True)
    target_dir = cargo_target_directory(root, environment)
    evidence = root / ".evidence"
    evidence.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="css-wasm-", dir=evidence) as temporary:
        stage = Path(temporary) / "new"
        stage.mkdir()
        _, bindgen, optimizer = cached_wasm_plan(target, target_dir, stage, *tools)
        subprocess.run(bindgen, cwd=root, env=environment, check=True)
        optimizer_environment = dict(environment)
        optimizer_environment.setdefault("BINARYEN_CORES", "1")
        subprocess.run(optimizer, cwd=root, env=optimizer_environment, check=True)
        (stage / "turbo_dom_css_optimized.wasm").replace(stage / "turbo_dom_css_bg.wasm")
        write_cached_manifest(root, stage, target)
        for suffix in WASM_SUFFIXES: file_record(stage, Path(f"turbo_dom_css{suffix}"))
        replace_wasm_package(stage, root / ("pkg-css" if target == "nodejs" else "pkg-css-web"))


def native_files():
    parser = [Path(f"turbo-dom-parser.{platform}.node") for platform in TARGETS.values()]
    css = [Path("native-css") / f"turbo-dom-css.{platform}.node" for platform in TARGETS.values()]
    return parser + css


def require_native_set(root, expected):
    actual = set(root.glob("*.node")) | set((root / "native-css").glob("*.node"))
    wanted = {root / path for path in expected}
    if actual != wanted:
        missing = sorted(str(path.relative_to(root)) for path in wanted - actual)
        extra = sorted(str(path.relative_to(root)) for path in actual - wanted)
        raise ValueError(f"native payload mismatch: missing={missing}, unexpected={extra}")


def file_record(root, relative):
    path = root / relative
    if not path.is_file() or path.stat().st_size == 0:
        raise ValueError(f"missing or empty artifact: {relative}")
    data = path.read_bytes()
    if path.suffix == ".wasm" and data[:8] != b"\x00asm\x01\x00\x00\x00":
        raise ValueError(f"invalid WASM header: {relative}")
    return {"path": str(relative), "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def check(root, version):
    require_native_set(root, native_files())
    files = native_files()
    for directory, stem in WASM_PACKAGES.items():
        manifest = Path(directory) / "package.json"
        file_record(root, manifest)
        package = json.loads((root / manifest).read_text())
        if package.get("version") != version:
            raise ValueError(f"{manifest}: version {package.get('version')!r}, expected {version}")
        files.append(manifest)
        files.extend(Path(directory) / f"{stem}{suffix}" for suffix in WASM_SUFFIXES)
        files.extend(path.relative_to(root) for path in (root / directory / "snippets").rglob("*.js"))
    return [file_record(root, path) for path in files]


def bundle(artifacts, root):
    expected = native_files()
    by_name = {}
    for path in artifacts.rglob("*.node"):
        by_name.setdefault(path.name, []).append(path)
    if set(by_name) != {path.name for path in expected} or any(len(paths) != 1 for paths in by_name.values()):
        raise ValueError("CI artifacts must contain exactly one of every parser/CSS native binary, and no extras")
    if list(root.glob("*.node")) or list((root / "native-css").glob("*.node")):
        raise ValueError("package already contains native binaries; refusing to replace reviewed artifacts")
    for relative in expected:
        file_record(artifacts, by_name[relative.name][0].relative_to(artifacts))
    for relative in expected:
        destination = root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(by_name[relative.name][0], destination)


def host_target():
    text = subprocess.check_output(["rustc", "-vV"], text=True)
    return next(line.removeprefix("host: ") for line in text.splitlines() if line.startswith("host: "))


def native_environment(target, inherited):
    environment = dict(inherited)
    # Match napi-rs' existing musl cdylib policy; a statically linked musl cdylib
    # cannot be loaded as a Node addon. Preserve every caller-provided flag.
    flags = environment.get("RUSTFLAGS", "")
    if "musl" in target and "target-feature=-crt-static" not in flags:
        environment["RUSTFLAGS"] = (flags + " -C target-feature=-crt-static").strip()
    return environment


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("native", "nodejs", "web", "plan", "check", "bundle"))
    parser.add_argument("--target")
    parser.add_argument("--artifacts", type=Path)
    args = parser.parse_args()
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", os.environ.get("CARGO_BUILD_TARGET_DIR", REPO / "target")))
    if not target_dir.is_absolute(): target_dir = REPO / target_dir
    if args.command == "plan":
        if not args.target: parser.error("plan requires an explicit --target; no compiler tools are invoked")
        command, source, destination = native_plan(args.target, target_dir)
        tools = cached_tools(os.environ)
        wasm = [cached_wasm_plan(target, target_dir, REPO / ".evidence/css-wasm-plan", *tools)
                for target in ("nodejs", "web")] if tools else [wasm_plan("nodejs"), wasm_plan("web")]
        print(json.dumps({"native": command, "sourceHint": str(source),
                          "sourceResolution": "locked Cargo metadata target_directory after build",
                          "destination": str(destination), "wasm": wasm,
                          "wasmTooling": "explicit cached executables" if tools else "wasm-pack"}, indent=2))
    elif args.command == "check":
        version = json.loads((REPO / "package.json").read_text())["version"]
        print(json.dumps({"version": version, "artifacts": check(REPO, version)}, indent=2))
    elif args.command == "bundle":
        if args.artifacts is None: parser.error("bundle requires --artifacts")
        bundle(args.artifacts, REPO)
    elif args.command == "native":
        target = args.target or host_target()
        command, source, destination = native_plan(target, target_dir)
        environment = native_environment(target, os.environ)
        subprocess.run(command, cwd=REPO, env=environment, check=True)
        _, source, destination = native_plan(target, cargo_target_directory(REPO, environment))
        destination = REPO / destination
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
    else:
        tools = cached_tools(os.environ)
        if tools:
            build_cached_wasm(REPO, args.command, tools, dict(os.environ))
        else:
            subprocess.run(wasm_plan(args.command), cwd=REPO, check=True)
            normalize_wasm_manifest(args.command, REPO)


if __name__ == "__main__":
    main()
