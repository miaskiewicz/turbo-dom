"""Offline checks for the turbo-dom-v8 release script (no network, no publishing).

Run: python3 -m unittest discover -s crates/turbo-dom-v8/scripts -p 'test_*.py'
"""
import importlib.util
import io
import tarfile
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("turbo_dom_v8_release", SCRIPTS / "release.py")
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)
PREFIX = f"{RELEASE.NAME}-{RELEASE.VERSION}/"
METADATA = ('{"version":{"crate":"%s","num":"%s"}}' % (RELEASE.NAME, RELEASE.VERSION)).encode()


def package_bytes(changes=None):
    files = {"Cargo.toml": f'[package]\nname="{RELEASE.NAME}"\nversion="{RELEASE.VERSION}"\n'.encode(),
             "src/isolate.rs": b"reviewed runtime", "src/function.rs": b"reviewed interceptor",
             "LICENSE": b"MIT license"}
    files.update(changes or {})
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w:gz") as package:
        for name, data in files.items():
            if data is None:
                continue
            member = tarfile.TarInfo(PREFIX + name)
            member.size = len(data)
            package.addfile(member, io.BytesIO(data))
    return stream.getvalue()


class ReleaseTurboDomV8Tests(unittest.TestCase):
    def test_identity_comes_from_pins(self):
        self.assertEqual((RELEASE.NAME, RELEASE.VERSION), ("turbo-dom-v8", "150.4.0"))

    def test_published_runtime_manifest_or_license_changes_are_rejected(self):
        for changes in [{"src/isolate.rs": b"unpatched runtime"},
                        {"src/function.rs": b"stock interceptor"},
                        {"Cargo.toml": f'[package]\nname="{RELEASE.NAME}"\nversion="{RELEASE.VERSION}"\nlinks="x"\n'.encode()},
                        {"LICENSE": b"other terms"},
                        {"src/extra.rs": b"unexpected"}]:
            with self.assertRaises(ValueError):
                RELEASE.require_same_distribution(package_bytes(), package_bytes(changes))

    def test_only_generated_lock_and_vcs_metadata_can_differ(self):
        RELEASE.require_same_distribution(
            package_bytes({"Cargo.lock": b"old resolution", ".cargo_vcs_info.json": b"old commit"}),
            package_bytes({"Cargo.lock": b"new resolution", ".cargo_vcs_info.json": b"new commit"}),
        )

    def test_wrong_identity_or_missing_reviewed_sources_are_rejected(self):
        for changes in [{"Cargo.toml": f'[package]\nname="v8"\nversion="{RELEASE.VERSION}"\n'.encode()},
                        {"Cargo.toml": f'[package]\nname="{RELEASE.NAME}"\nversion="150.4.1"\n'.encode()},
                        {"LICENSE": None}, {"src/function.rs": None}]:
            with self.assertRaises((ValueError, KeyError)):
                RELEASE.archive_payload(package_bytes(changes))

    def test_unsafe_archive_paths_are_rejected(self):
        with self.assertRaises(ValueError):
            RELEASE.archive_payload(package_bytes({"../escape": b"unexpected"}))

    def test_missing_version_api_404_does_not_request_a_static_archive(self):
        error = urllib.error.HTTPError("api", 404, "missing", {}, None)
        with mock.patch.object(RELEASE.urllib.request, "urlopen", side_effect=error) as request:
            self.assertIsNone(RELEASE.published_archive())
        self.assertEqual(request.call_count, 1)
        self.assertEqual(request.call_args.args[0].full_url,
                         "https://crates.io/api/v1/crates/turbo-dom-v8/150.4.0")

    def test_present_version_downloads_the_exact_static_archive(self):
        archive = package_bytes()
        with mock.patch.object(RELEASE.urllib.request, "urlopen",
                               side_effect=[io.BytesIO(METADATA), io.BytesIO(archive)]) as request:
            self.assertEqual(RELEASE.published_archive(), archive)
        self.assertEqual(request.call_count, 2)
        self.assertEqual(request.call_args.args[0].full_url,
                         "https://static.crates.io/crates/turbo-dom-v8/turbo-dom-v8-150.4.0.crate")

    def test_api_or_existing_archive_403_remains_a_failure(self):
        for after_metadata in [False, True]:
            error = urllib.error.HTTPError("denied", 403, "denied", {}, None)
            calls = [error]
            if after_metadata:
                calls.insert(0, io.BytesIO(METADATA))
            with mock.patch.object(RELEASE.urllib.request, "urlopen", side_effect=calls):
                with self.assertRaises(urllib.error.HTTPError):
                    RELEASE.published_archive()
            error.close()

    def test_wrong_registry_version_is_rejected_before_archive_download(self):
        metadata = io.BytesIO(b'{"version":{"crate":"v8","num":"150.4.0"}}')
        with mock.patch.object(RELEASE.urllib.request, "urlopen", return_value=metadata) as request:
            with self.assertRaises(ValueError):
                RELEASE.published_archive()
        self.assertEqual(request.call_count, 1)

    def test_publishing_requires_turbo_dom_tag_repository_and_generated_crate(self):
        tag = "refs/tags/v" + RELEASE.release_version()
        crate = RELEASE.CRATE / "crate"
        good = {"GITHUB_REPOSITORY": "miaskiewicz/turbo-dom", "GITHUB_REF": tag}
        with mock.patch.dict(RELEASE.os.environ, good, clear=True):
            RELEASE.verify_publish_context(crate)
            with tempfile.TemporaryDirectory() as other:
                with self.assertRaises(ValueError):
                    RELEASE.verify_publish_context(Path(other))
        for key, value in [("GITHUB_REF", "refs/heads/main"), ("GITHUB_REF", "refs/tags/v150.4.0"),
                           ("GITHUB_REPOSITORY", "miaskiewicz/turbo-surf")]:
            with mock.patch.dict(RELEASE.os.environ, dict(good, **{key: value}), clear=True):
                with self.assertRaises(ValueError):
                    RELEASE.verify_publish_context(crate)

    def test_publish_refuses_source_replacement_before_packaging(self):
        with mock.patch.object(RELEASE, "verify_publish_context"), \
                mock.patch.object(RELEASE, "prepare") as prepare:
            for extra in [["--offline"], ["--config", "x.toml"]]:
                with self.assertRaises(SystemExit):
                    RELEASE.main(["--out", "unused", "--publish", *extra])
            prepare.assert_not_called()

    def test_existing_identical_publication_is_verified_not_reuploaded(self):
        archive = package_bytes()
        with tempfile.TemporaryDirectory() as temporary:
            out = Path(temporary) / "out"
            package = Path(temporary) / "pkg.crate"
            package.write_bytes(archive)
            with mock.patch.object(RELEASE, "verify_publish_context"), \
                    mock.patch.object(RELEASE, "prepare", return_value=(out, package, {})), \
                    mock.patch.object(RELEASE, "published_archive", return_value=archive), \
                    mock.patch.object(RELEASE.subprocess, "run") as run, \
                    mock.patch("builtins.print"):
                out.mkdir()
                RELEASE.main(["--out", str(out), "--publish"])
            run.assert_not_called()
            receipt = (out / "receipt.json").read_text()
            self.assertIn('"existingPublishedSourceVerified": true', receipt)
            self.assertIn('"published": false', receipt)


if __name__ == "__main__":
    unittest.main()
