#!/usr/bin/env python3
"""Publish the turbo-dom workspace crates in dependency order (release workflow helper).

  publish-crates.py wait-index NAME VERSION          poll the crates.io index for NAME VERSION
  publish-crates.py drop-v8-patch                    resolve turbo-dom-v8 from crates.io
  publish-crates.py publish [--dry-run]              turbo-dom -> -layout -> -raster -> -browser-env

`publish` is idempotent: a crate whose version is already on crates.io is skipped
(registry versions are immutable), every other crate is published with
`cargo publish -p NAME` (Cargo's packaged-consumer verification stays on) and the
next crate starts only after the previous version is listed in the crates.io sparse
index, so its verification build resolves it. turbo-dom-v8 publishes before this, from
its generated crate, through crates/turbo-dom-v8/scripts/release.py.

`drop-v8-patch` removes the root manifest's pre-publish `[patch.crates-io]` (the
turbo-dom-v8 candidate branch) and re-locks only turbo-dom-v8, so turbo-dom-browser-env
is verified against the published fork. `--dry-run` runs `cargo package --list` per
crate instead of publishing (no network writes).
"""
import argparse
import json
import subprocess
import sys
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
ORDER = ["turbo-dom", "turbo-dom-layout", "turbo-dom-raster", "turbo-dom-browser-env"]
USER_AGENT = "turbo-dom-release (https://github.com/miaskiewicz/turbo-dom)"


def index_path(name):
    """The crates.io sparse-index path of a crate name."""
    name = name.lower()
    if len(name) <= 2:
        return f"{len(name)}/{name}"
    if len(name) == 3:
        return f"3/{name[0]}/{name}"
    return f"{name[:2]}/{name[2:4]}/{name}"


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Cache-Control": "no-cache"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.read()
    except urllib.error.HTTPError as error:
        error.close()
        if error.code in (403, 404):
            return None
        raise


def indexed_versions(name, fetch=fetch):
    body = fetch(f"https://index.crates.io/{index_path(name)}")
    if body is None:
        return set()
    return {json.loads(line)["vers"] for line in body.decode().splitlines() if line.strip()}


def published(name, version, fetch=fetch):
    """Whether crates.io has NAME VERSION (its API is authoritative, the index may lag)."""
    body = fetch(f"https://crates.io/api/v1/crates/{name}/{version}")
    if body is None:
        return False
    data = json.loads(body)["version"]
    if data.get("crate") != name or data.get("num") != version:
        raise ValueError(f"crates.io returned {data.get('crate')} {data.get('num')} for {name} {version}")
    return True


def wait_index(name, version, attempts=60, delay=10, fetch=fetch, sleep=time.sleep):
    for attempt in range(attempts):
        if version in indexed_versions(name, fetch):
            print(f"{name} {version} is in the crates.io index", flush=True)
            return
        if attempt + 1 < attempts:
            sleep(delay)
    raise RuntimeError(f"{name} {version} did not appear in the crates.io index")


def strip_patch_section(text):
    """Remove the `[patch.crates-io]` table and the comment block directly above it."""
    lines = text.splitlines(keepends=True)
    try:
        start = next(i for i, line in enumerate(lines) if line.strip() == "[patch.crates-io]")
    except StopIteration:
        return text
    end = start + 1
    while end < len(lines) and not lines[end].lstrip().startswith("["):
        end += 1
    while start > 0 and lines[start - 1].lstrip().startswith("#"):
        start -= 1
    while start > 0 and not lines[start - 1].strip():
        start -= 1
    return "".join(lines[:start]) + ("\n" if end < len(lines) else "") + "".join(lines[end:])


def drop_v8_patch(cargo="cargo"):
    manifest = REPO / "Cargo.toml"
    text = manifest.read_text()
    stripped = strip_patch_section(text)
    if stripped == text:
        print("no [patch.crates-io] in Cargo.toml")
        return
    if "patch" in tomllib.loads(stripped):
        raise ValueError("Cargo.toml still has a patch table")
    manifest.write_text(stripped)
    subprocess.run([cargo, "update", "-p", "turbo-dom-v8"], cwd=REPO, check=True)
    tree = subprocess.run([cargo, "tree", "-p", "turbo-dom-browser-env", "-i", "turbo-dom-v8", "--depth", "0"],
                          cwd=REPO, check=True, capture_output=True, text=True).stdout
    print(tree, end="")
    if "(" in tree.splitlines()[0]:  # registry packages print no "(source)" suffix
        raise RuntimeError("turbo-dom-v8 still resolves from a git/path source")


def workspace_version(name):
    return tomllib.loads((REPO / "crates" / name / "Cargo.toml").read_text())["package"]["version"]


def publish(dry_run=False, cargo="cargo"):
    for name in ORDER:
        version = workspace_version(name)
        if dry_run:
            subprocess.run([cargo, "package", "-p", name, "--list", "--allow-dirty"], cwd=REPO, check=True,
                           stdout=subprocess.DEVNULL)
            print(f"{name} {version}: package list ok (dry run)")
            continue
        if published(name, version):
            print(f"{name} {version} is already on crates.io; skipping", flush=True)
        else:
            print(f"publishing {name} {version}", flush=True)
            subprocess.run([cargo, "publish", "-p", name, "--allow-dirty"], cwd=REPO, check=True)
        wait_index(name, version)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    wait = sub.add_parser("wait-index")
    wait.add_argument("name")
    wait.add_argument("version")
    sub.add_parser("drop-v8-patch")
    pub = sub.add_parser("publish")
    pub.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    if args.command == "wait-index":
        wait_index(args.name, args.version)
    elif args.command == "drop-v8-patch":
        drop_v8_patch()
    else:
        publish(args.dry_run)


if __name__ == "__main__":
    sys.exit(main())
