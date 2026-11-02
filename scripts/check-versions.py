#!/usr/bin/env python3
"""Check that every turbo-dom release version agrees (CI guard + publish preflight).

  check-versions.py [--tag vX.Y.Z]

- npm `package.json`, both root `package-lock.json` entries, the root
  `turbo-dom-parser` crate and the four workspace crates (turbo-dom, -layout, -raster,
  -browser-env) carry the same version;
- every intra-workspace path dependency also names that version (`version = "X.Y.Z"`,
  required to publish), and turbo-dom-browser-env requires turbo-dom-v8 `~<pins
  version without patch>`; `crates/turbo-dom-v8/pins.json` keeps its upstream-derived
  version;
- README's "Release **X.Y.Z**" line names it;
- with --tag, the tag equals `v<version>` (the publish workflow passes its tag).
"""
import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
CRATES = ["turbo-dom", "turbo-dom-layout", "turbo-dom-raster", "turbo-dom-browser-env"]


def manifest(path):
    return tomllib.loads((REPO / path).read_text())


def dependencies(toml):
    yield from toml.get("dependencies", {}).items()
    for target in toml.get("target", {}).values():
        yield from target.get("dependencies", {}).items()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--tag")
    args = parser.parse_args(argv)
    errors = []
    package = json.loads((REPO / "package.json").read_text())
    version = package["version"]
    lock = json.loads((REPO / "package-lock.json").read_text())
    found = {
        "package-lock.json": lock["version"],
        'package-lock.json packages[""]': lock["packages"][""]["version"],
        "Cargo.toml (turbo-dom-parser)": manifest("Cargo.toml")["package"]["version"],
    }
    for crate in CRATES:
        toml = manifest(f"crates/{crate}/Cargo.toml")
        found[f"crates/{crate}"] = toml["package"]["version"]
        for name, spec in dependencies(toml):
            if isinstance(spec, dict) and "path" in spec:
                if spec.get("version") != version:
                    errors.append(f"crates/{crate}: path dependency {name} needs version = \"{version}\""
                                  f" (has {spec.get('version')!r})")
    for where, value in found.items():
        if value != version:
            errors.append(f"{where} is {value}, package.json is {version}")
    pins = json.loads((REPO / "crates/turbo-dom-v8/pins.json").read_text())
    v8_req = "~" + ".".join(pins["version"].split(".")[:2])
    env_v8 = dict(dependencies(manifest("crates/turbo-dom-browser-env/Cargo.toml")))["v8"]
    if env_v8.get("package") != "turbo-dom-v8" or env_v8.get("version") != v8_req:
        errors.append(f"turbo-dom-browser-env must depend on turbo-dom-v8 {v8_req} (has {env_v8})")
    if not re.search(rf"^Release \*\*{re.escape(version)}\*\*", (REPO / "README.md").read_text(), re.M):
        errors.append(f"README.md has no 'Release **{version}**' line")
    if args.tag is not None and args.tag != f"v{version}":
        errors.append(f"tag {args.tag} does not match version {version} (expected v{version})")
    for line in errors:
        print(f"check-versions: {line}")
    if errors:
        sys.exit(1)
    print(f"check-versions: {version} everywhere (turbo-dom-v8 {pins['version']})")


if __name__ == "__main__":
    main()
