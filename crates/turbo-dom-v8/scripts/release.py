#!/usr/bin/env python3
"""Package the generated turbo-dom-v8 crate; publish only from turbo-dom's version tag.

Idempotent by crate name and version: when turbo-dom-v8 150.4.0 already exists on
crates.io, the published archive must equal the freshly packaged one (only Cargo's
generated Cargo.lock / .cargo_vcs_info.json may differ) and nothing is uploaded.

  release.py --out DIR [--crate DIR] [--offline] [--config FILE]   package + verify only
  release.py --out DIR --publish                                     CI: refs/tags/v<turbo-dom>

The crate directory defaults to crates/turbo-dom-v8/crate/ (written by generate.py) and
must match pins.json before anything is packaged.
"""
import argparse
import hashlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import tarfile
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path, PurePosixPath

SCRIPTS = Path(__file__).resolve().parent
CRATE = SCRIPTS.parent
REPO = CRATE.parents[1]
_SPEC = importlib.util.spec_from_file_location("turbo_dom_v8_generate", SCRIPTS / "generate.py")
GENERATE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(GENERATE)
PINS = GENERATE.load_pins()
NAME = PINS["package"]
VERSION = PINS["version"]
REPOSITORY = "miaskiewicz/turbo-dom"
GENERATED_RECORDS = {"Cargo.lock", ".cargo_vcs_info.json"}


def archive_payload(raw):
    payload = {}
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
        for member in archive:
            if member.isdir():
                continue
            path = PurePosixPath(member.name)
            if (not member.isfile() or path.is_absolute() or ".." in path.parts
                    or not path.parts or path.parts[0] != f"{NAME}-{VERSION}"):
                raise ValueError("unsafe or unexpected package member")
            relative = str(PurePosixPath(*path.parts[1:]))
            if relative in payload:
                raise ValueError("duplicate package member")
            if relative not in GENERATED_RECORDS:
                payload[relative] = archive.extractfile(member).read()
    manifest = tomllib.loads(payload["Cargo.toml"].decode())
    if manifest["package"]["name"] != NAME or manifest["package"]["version"] != VERSION:
        raise ValueError("wrong published fork identity")
    for required in ["LICENSE", "src/isolate.rs", "src/function.rs"]:
        if required not in payload:
            raise ValueError("missing license or reviewed runtime source: " + required)
    return payload


def require_same_distribution(expected, actual):
    left, right = archive_payload(expected), archive_payload(actual)
    different = sorted(path for path in left.keys() | right.keys()
                       if left.get(path) != right.get(path))
    if different:
        raise ValueError("published fork differs: " + ", ".join(different))


def prepare(output, crate, offline=False, config=None):
    GENERATE.verify_output(crate, PINS)
    output.mkdir(parents=True, exist_ok=False)
    source = output / "source"
    shutil.copytree(crate, source, ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    command = ["cargo", "package", "--manifest-path", str(source / "Cargo.toml"),
               "--allow-dirty", "--no-verify", "--exclude-lockfile"]
    if offline:
        command.append("--offline")
    if config:
        command += ["--config", str(config)]
    env = dict(os.environ, CARGO_TARGET_DIR=str(output / "target"))
    with (output / "package.log").open("w") as log:
        subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    package = output / "target/package" / f"{NAME}-{VERSION}.crate"
    archive_payload(package.read_bytes())
    return source, package, env


def registry_request(url):
    return urllib.request.Request(url, headers={"User-Agent": "turbo-dom-v8-release"})


def published_archive():
    # Missing static objects can return 403. Only the registry's version API can
    # establish absence; a denied archive for an existing version must stay fatal.
    api = f"https://crates.io/api/v1/crates/{NAME}/{VERSION}"
    try:
        with urllib.request.urlopen(registry_request(api), timeout=30) as response:
            metadata = json.load(response)["version"]
    except urllib.error.HTTPError as error:
        error.close()
        if error.code == 404:
            return None
        raise
    if metadata.get("crate") != NAME or metadata.get("num") != VERSION:
        raise ValueError("registry returned a different fork identity")
    url = f"https://static.crates.io/crates/{NAME}/{NAME}-{VERSION}.crate"
    with urllib.request.urlopen(registry_request(url), timeout=30) as response:
        return response.read()


def release_version(repo=REPO):
    return tomllib.loads((repo / "crates/turbo-dom/Cargo.toml").read_text())["package"]["version"]


def verify_publish_context(crate, repo=REPO):
    """Publish only from this repository's turbo-dom version tag and its generated crate."""
    expected = {"GITHUB_REPOSITORY": REPOSITORY, "GITHUB_REF": "refs/tags/v" + release_version(repo)}
    if any(os.environ.get(key) != value for key, value in expected.items()):
        raise ValueError("publishing requires the matching turbo-dom version tag in " + REPOSITORY)
    if crate.resolve() != (CRATE / "crate").resolve():
        raise ValueError("publishing requires the crate generated in crates/turbo-dom-v8/crate")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--crate", type=Path, default=CRATE / "crate")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--config", type=Path)
    parser.add_argument("--publish", action="store_true")
    args = parser.parse_args(argv)
    crate = args.crate.resolve()
    if args.publish:
        verify_publish_context(crate)
        if args.offline or args.config:
            parser.error("publishing must use the real default registry without source replacement")
    output = args.out.resolve()
    source, package, env = prepare(output, crate, args.offline, args.config)
    expected = package.read_bytes()
    receipt = {"name": NAME, "version": VERSION, "upstreamVersion": PINS["upstream"]["version"],
               "generatedTreeSha256": PINS["generated"]["tree_sha256"],
               "archiveSha256": hashlib.sha256(expected).hexdigest(), "archiveBytes": len(expected),
               "published": False, "existingPublishedSourceVerified": False}
    if args.publish:
        actual = published_archive()
        if actual is not None:
            require_same_distribution(expected, actual)
            receipt["existingPublishedSourceVerified"] = True
        else:
            # Keep Cargo's actual packaged-consumer verification enabled.
            subprocess.run(["cargo", "publish", "--manifest-path", str(source / "Cargo.toml"),
                            "--allow-dirty"], env=env, check=True)
            for _ in range(40):
                actual = published_archive()
                if actual is not None:
                    require_same_distribution(expected, actual)
                    receipt["published"] = True
                    break
                time.sleep(3)
            else:
                raise RuntimeError("published fork archive not observable within 120 seconds")
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
