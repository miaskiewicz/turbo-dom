"""Offline packaging tests: no compiler, network, Node or binary execution."""
import importlib.util
import contextlib
import io
import os
import sys
import json
import tempfile
import tomllib
import unittest
from unittest import mock
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("css_artifacts", HERE / "css-artifacts.py")
ca = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ca)


class BuildPlans(unittest.TestCase):
    def test_native_uses_separate_output_and_explicit_features(self):
        command, source, destination = ca.native_plan("aarch64-apple-darwin", Path("/cache"))
        self.assertEqual(command, ["cargo", "build", "--locked", "--release", "--no-default-features", "--features", "napi-bind,css-backend", "--target", "aarch64-apple-darwin"])
        self.assertEqual(source, Path("/cache/aarch64-apple-darwin/release/libturbo_dom_parser.dylib"))
        self.assertEqual(destination, Path("native-css/turbo-dom-css.darwin-arm64.node"))

    def test_native_platform_names_and_unsupported_target(self):
        for target, platform in ca.TARGETS.items():
            _, _, output = ca.native_plan(target, Path("target"))
            self.assertEqual(output.name, f"turbo-dom-css.{platform}.node")
        with self.assertRaises(ValueError):
            ca.native_plan("invented-target", Path("target"))

    def test_native_musl_flags_preserve_caller_flags_and_do_not_change_other_targets(self):
        source = {"RUSTFLAGS": "-C strip=symbols", "CARGO_BUILD_JOBS": "1"}
        result = ca.native_environment("x86_64-unknown-linux-musl", source)
        self.assertEqual(result["RUSTFLAGS"], "-C strip=symbols -C target-feature=-crt-static")
        self.assertEqual(result["CARGO_BUILD_JOBS"], "1")
        self.assertEqual(ca.native_environment("x86_64-unknown-linux-musl", result), result)
        self.assertEqual(ca.native_environment("aarch64-apple-darwin", source), source)
        self.assertEqual(source["RUSTFLAGS"], "-C strip=symbols")

    def test_css_wasm_packages_get_explicit_module_types_without_version_changes(self):
        for target, directory, module_type in [("nodejs", "pkg-css", "commonjs"), ("web", "pkg-css-web", "module")]:
            with tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                folder = root / directory
                folder.mkdir()
                (folder / "package.json").write_text('{"version":"0.8.7","main":"original.js"}')
                ca.normalize_wasm_manifest(target, root)
                package = json.loads((folder / "package.json").read_text())
                self.assertEqual(package, {"version": "0.8.7", "main": "original.js", "private": True, "type": module_type})

    def test_wasm_outputs_do_not_replace_parser_packages(self):
        for target, directory in [("nodejs", "pkg-css"), ("web", "pkg-css-web")]:
            self.assertEqual(ca.wasm_plan(target), ["wasm-pack", "build", "--target", target, "--out-dir", directory, "--out-name", "turbo_dom_css", "--no-default-features", "--features", "wasm-bind,css-backend"])
        with self.assertRaises(ValueError):
            ca.wasm_plan("invalid")


class CachedToolPlans(unittest.TestCase):
    def test_postbuild_metadata_target_directory_overrides_implicit_repo_target(self):
        runner = mock.Mock(return_value=json.dumps({"target_directory": "/configured/cargo/cache"}))
        directory = ca.cargo_target_directory(Path("/repo"), {"EXAMPLE": "unchanged"}, runner)
        self.assertEqual(directory, Path("/configured/cargo/cache"))
        self.assertEqual(ca.native_plan("aarch64-apple-darwin", directory)[1],
                         Path("/configured/cargo/cache/aarch64-apple-darwin/release/libturbo_dom_parser.dylib"))
        self.assertEqual(runner.call_args.args[0], ["cargo", "metadata", "--offline", "--locked", "--no-deps", "--format-version", "1"])
        self.assertEqual(runner.call_args.kwargs["env"], {"EXAMPLE": "unchanged"})
        for invalid in ({}, {"target_directory": "relative"}):
            with self.assertRaises(ValueError):
                ca.cargo_target_directory(Path("/repo"), {}, mock.Mock(return_value=json.dumps(invalid)))

    def test_native_cli_copies_from_real_postbuild_metadata_not_a_guessed_directory(self):
        with tempfile.TemporaryDirectory() as temp:
            root, configured = Path(temp) / "repo", Path(temp) / "configured"
            root.mkdir()
            with mock.patch.object(ca, "REPO", root), mock.patch.dict(os.environ, {}, clear=True), \
                    mock.patch.object(sys, "argv", ["css-artifacts.py", "native", "--target", "aarch64-apple-darwin"]), \
                    mock.patch.object(ca.subprocess, "run") as build, \
                    mock.patch.object(ca, "cargo_target_directory", return_value=configured), \
                    mock.patch.object(ca.shutil, "copyfile") as copy:
                ca.main()
            build.assert_called_once()
            copy.assert_called_once_with(configured / "aarch64-apple-darwin/release/libturbo_dom_parser.dylib",
                                         root / "native-css/turbo-dom-css.darwin-arm64.node")

    def test_plan_prints_cached_commands_without_invoking_compilers_or_tools(self):
        environment = {"TURBO_DOM_WASM_BINDGEN": "/cache/bindgen", "TURBO_DOM_WASM_OPT": "/cache/opt"}
        output = io.StringIO()
        with mock.patch.dict(os.environ, environment, clear=True), \
                mock.patch.object(sys, "argv", ["css-artifacts.py", "plan", "--target", "aarch64-apple-darwin"]), \
                mock.patch.object(ca.subprocess, "run", side_effect=AssertionError("compiler invoked")), \
                mock.patch.object(ca.subprocess, "check_output", side_effect=AssertionError("tool invoked")), \
                contextlib.redirect_stdout(output):
            ca.main()
        plan = json.loads(output.getvalue())
        self.assertEqual(plan["wasmTooling"], "explicit cached executables")
        self.assertEqual(plan["wasm"][0][1][0], "/cache/bindgen")
        self.assertIn("metadata", plan["sourceResolution"])

    def test_explicit_cached_tools_never_install_or_fallback(self):
        env = {"TURBO_DOM_WASM_BINDGEN": "/cache/wasm-bindgen", "TURBO_DOM_WASM_OPT": "/cache/wasm-opt"}
        self.assertEqual(ca.cached_tools(env), ("/cache/wasm-bindgen", "/cache/wasm-opt"))
        self.assertIsNone(ca.cached_tools({}))
        with self.assertRaises(ValueError): ca.cached_tools({"TURBO_DOM_WASM_BINDGEN": "/cache/wasm-bindgen"})
        with self.assertRaises(ValueError): ca.cached_tools({"TURBO_DOM_WASM_OPT": "/cache/wasm-opt"})
        plan = ca.cached_wasm_plan("nodejs", Path("/cargo"), Path("/stage"), *ca.cached_tools(env))
        self.assertEqual(plan[0], ["cargo", "build", "--offline", "--locked", "--release", "--no-default-features", "--features", "wasm-bind,css-backend", "--target", "wasm32-unknown-unknown"])
        self.assertEqual(plan[1], ["/cache/wasm-bindgen", "/cargo/wasm32-unknown-unknown/release/turbo_dom_parser.wasm", "--target", "nodejs", "--out-dir", "/stage", "--out-name", "turbo_dom_css"])
        self.assertEqual(plan[2], ["/cache/wasm-opt", "-Oz", "/stage/turbo_dom_css_bg.wasm", "-o", "/stage/turbo_dom_css_optimized.wasm"])

    def test_tool_versions_must_match_the_locked_bindgen_and_reviewed_optimizer(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "Cargo.lock").write_text('version=4\n[[package]]\nname="wasm-bindgen"\nversion="0.2.122"\n')
            runner = mock.Mock(side_effect=["wasm-bindgen 0.2.122", "wasm-opt version 117 (version_117)"])
            ca.verify_cached_tools(root, ("bindgen", "optimizer"), runner)
            for results in (["wasm-bindgen 0.2.125"], ["wasm-bindgen 0.2.122", "wasm-opt version 999"]):
                with self.assertRaises(ValueError): ca.verify_cached_tools(root, ("bindgen", "optimizer"), mock.Mock(side_effect=results))

    def test_generated_cached_package_has_explicit_type_version_and_complete_files(self):
        with tempfile.TemporaryDirectory() as temp:
            root, stage = Path(temp) / "root", Path(temp) / "stage"
            root.mkdir(); stage.mkdir()
            (root / "package.json").write_text('{"version":"0.8.7"}')
            ca.write_cached_manifest(root, stage, "web")
            package = json.loads((stage / "package.json").read_text())
            self.assertEqual(package["version"], "0.8.7")
            self.assertEqual(package["type"], "module")
            self.assertTrue(package["private"])
            self.assertEqual(package["module"], "turbo_dom_css.js")
            self.assertIn("turbo_dom_css_bg.wasm", package["files"])

    def test_cached_package_replacement_removes_stale_files_and_restores_on_failure(self):
        for fail in (False, True):
            with self.subTest(fail=fail), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                stage, destination = root / "new", root / "pkg-css"
                stage.mkdir(); destination.mkdir()
                (stage / "new.js").write_text("new")
                (destination / "stale.js").write_text("old")
                original = Path.rename
                def rename(path, target):
                    if fail and path == stage: raise OSError("test publication failure")
                    return original(path, target)
                with mock.patch.object(Path, "rename", new=rename):
                    if fail:
                        with self.assertRaises(OSError): ca.replace_wasm_package(stage, destination)
                    else: ca.replace_wasm_package(stage, destination)
                self.assertTrue((destination / ("stale.js" if fail else "new.js")).is_file())
                self.assertFalse((destination / ("new.js" if fail else "stale.js")).exists())


class SourceContracts(unittest.TestCase):
    def test_parser_defaults_remain_independent_and_css_feature_is_explicit(self):
        root = HERE.parent
        manifest = tomllib.loads((root / "Cargo.toml").read_text())
        self.assertEqual(manifest["features"]["default"], ["napi-bind"])
        self.assertNotIn("css-backend", manifest["features"]["napi-bind"])
        self.assertNotIn("css-backend", manifest["features"]["wasm-bind"])
        for alias in ("turbo_dom_core", "turbo_dom_layout", "serde_json"):
            self.assertTrue(manifest["dependencies"][alias]["optional"])
        self.assertIn("turbo_dom_core/computed-style-grammar", manifest["features"]["css-backend"])

    def test_full_js_gates_and_published_file_list_require_real_css_payload(self):
        package = json.loads((HERE.parent / "package.json").read_text())
        for script in ("test", "test:cov", "test:vitest", "test:jest"):
            self.assertTrue(package["scripts"][script].startswith("npm run check:css:host && "))
        self.assertNotIn("build:css", package["scripts"]["build"])
        self.assertIn("native-css/*.node", package["files"])
        for directory in ("pkg-css", "pkg-css-web"):
            for suffix in (".js", ".d.ts"):
                self.assertIn(f"{directory}/turbo_dom_css{suffix}", package["files"])
            self.assertNotIn(f"{directory}/turbo_dom_css_bg.wasm", package["files"])
        self.assertIn("css-wasm/*", package["files"])
        self.assertEqual(package["exports"]["./css-wasm"]["import"], "./pkg-css-web/turbo_dom_css.js")


class ArtifactChecks(unittest.TestCase):
    def populate(self, root):
        for path in ca.native_files():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_bytes(b"native")
        for directory, stem in ca.WASM_PACKAGES.items():
            folder = root / directory
            folder.mkdir()
            (folder / "package.json").write_text(json.dumps({"version": "0.8.7", "types": f"{stem}.d.ts"}))
            for suffix in ca.WASM_SUFFIXES:
                (folder / f"{stem}{suffix}").write_bytes(b"\x00asm\x01\x00\x00\x00" if suffix == "_bg.wasm" else b"fixture")

    def test_complete_package_is_reported_with_sizes_and_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.populate(root)
            report = ca.check(root, "0.8.7")
            self.assertEqual(len(report), 30)
            self.assertTrue(all(row["bytes"] > 0 and len(row["sha256"]) == 64 for row in report))
            snippet = root / "pkg-css/snippets/generated/module.js"
            snippet.parent.mkdir(parents=True)
            snippet.write_text("export const generated=1")
            self.assertTrue(any(row["path"] == "pkg-css/snippets/generated/module.js" for row in ca.check(root, "0.8.7")))

    def test_missing_empty_wrong_version_and_extra_native_fail(self):
        for mutation in ("missing", "empty", "version", "extra", "bad-wasm"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.populate(root)
                target = root / "native-css/turbo-dom-css.darwin-arm64.node"
                if mutation == "missing": target.unlink()
                if mutation == "empty": target.write_bytes(b"")
                if mutation == "version": (root / "pkg-css/package.json").write_text('{"version":"0.8.6"}')
                if mutation == "extra": (root / "unreviewed.node").write_bytes(b"extra")
                if mutation == "bad-wasm": (root / "pkg-css/turbo_dom_css_bg.wasm").write_bytes(b"wrong")
                with self.assertRaises(ValueError): ca.check(root, "0.8.7")

    def test_bundle_requires_exact_expected_native_payload_and_preserves_layout(self):
        with tempfile.TemporaryDirectory() as temp:
            root, artifacts = Path(temp) / "package", Path(temp) / "artifacts"
            root.mkdir(); artifacts.mkdir()
            for path in ca.native_files():
                source = artifacts / "matrix" / path
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_bytes(b"native")
            ca.bundle(artifacts, root)
            self.assertTrue(all((root / path).is_file() for path in ca.native_files()))
            with self.assertRaises(ValueError): ca.bundle(artifacts, root)

    def test_bundle_rejects_missing_duplicate_and_unknown_artifacts_before_copy(self):
        for mutation in ("missing", "duplicate", "unknown"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temp:
                root, artifacts = Path(temp) / "package", Path(temp) / "artifacts"
                root.mkdir(); artifacts.mkdir()
                for path in ca.native_files(): (artifacts / path.name).write_bytes(b"native")
                if mutation == "missing": next(artifacts.glob("*.node")).unlink()
                if mutation == "duplicate":
                    (artifacts / "duplicate").mkdir()
                    (artifacts / "duplicate" / ca.native_files()[0].name).write_bytes(b"native")
                if mutation == "unknown": (artifacts / "unexpected.node").write_bytes(b"native")
                with self.assertRaises(ValueError): ca.bundle(artifacts, root)
                self.assertEqual(list(root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
