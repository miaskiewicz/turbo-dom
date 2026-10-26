#!/usr/bin/env bash
# Driver for the `selector-dual` gate (turbo-dom consolidation S3a/S3b; see
# docs/selector-dual-gate.md). Builds every in-scope artifact with the frozen B-main
# selector engine compiled in, installs them where the suites load them from, runs the
# suites in one engine mode, and checks the logs.
#
# usage: scripts/selector-dual-gate.sh <step> <legacy|new> [suite ...]
#
# suites (default: every suite whose checkout is configured):
#   turbo-dom          cargo tests of turbo-dom + turbo-dom-browser-env (this checkout)
#   turbo-test-cargo   turbo-test `cargo test --locked`        } a scratch COPY of TURBO_TEST_DIR
#   turbo-test-npm     turbo-test `npm test` (node --test)     } with its turbo-dom patches
#   turbo-test-compat  turbo-test fixtures/compat (json)       } pointed at this checkout
#   surf-cargo         turbo-surf `cargo test --workspace`      } run IN TURBO_SURF_DIR; installed
#   surf-shim          turbo-surf `npm run test:shim`           } files are restored on exit
#   surf-e2e           turbo-surf `npm run test:e2e`            }
#   surf-detection     turbo-surf `npm run test:detection`      }
#   surf-corpus        tools/parity `surf` + `surf-extra` corpora through the dual napi addon,
#                      compared with SURF_CORPUS_CHECKPOINT (needs tools/parity/fetch-inputs.sh)
# S3b (the html2pdf cascade on the engine; the legacy engine is the old html2pdf matcher):
#   layout             turbo-dom-layout + turbo-dom-raster cargo tests (this checkout), in the
#   layout-svg         configurations of the S2 gate: default, svg, no-default and every
#   layout-nodefault   feature listed explicitly (layout-allfeat: svg,perf,test-support)
#   layout-allfeat
#   html2pdf-ws        turbo-html2pdf `cargo test --workspace`          } in TURBO_HTML2PDF_DIR, whose
#   html2pdf-core-*    core all-features (listed) / no-default / svg    } turbo-dom patch must point
#   html2pdf-corpus    tools/parity html2pdf corpora through dual napi  } at this checkout
#                      addons (default + svg), compared with HTML2PDF_CORPUS_CHECKPOINT
#   surf-static        surf static screenshots (turbo-dom-raster) over the parity surf inputs,
#                      through SURF_STATIC_HARNESS (a cargo package whose binary renders every
#                      input given as arguments with this checkout's raster)
#
# environment:
#   TURBO_TEST_DIR   turbo-test checkout (read only; copied)   default ../turbo-test-consolidation
#   TURBO_SURF_DIR   turbo-surf checkout (surf-* suites only; must forward `selector-dual`)
#   OUT              output dir                                default $TMPDIR/selector-dual/<step>-<mode>
#   RUSTY_V8_ARCHIVE        V8 archive for turbo-dom-browser-env / turbo-test (150.4 default build)
#   RUSTY_V8_ARCHIVE_SURF   V8 archive for turbo-surf (150.4 simdutf build)
#   TT_FEATURES      turbo-test cargo features   default turbo-dom-browser-env/selector-dual
#   SURF_FEATURES    turbo-surf cargo features   default selector-dual (surf crates forward it)
#   SURF_NAPI_PACKAGE / SURF_NAPI_NODE  surf napi crate + the absolute .node path the
#                    playwright suites load (TURBO_SURF_NAPI_PATH)
#   SURF_DETECTION_PACKAGE  surf crate owning examples/detection_trace   default turbo-surf-mcp
#   TURBO_SURF_CHROME_PATH  passed through to surf-detection
#   SURF_CORPUS_CHECKPOINT  parity checkpoint for surf-corpus   default b2-surf
#   SURF_CORPUS_EXPECT      optional expected-diff file(s) for surf-corpus (space separated)
#   TURBO_HTML2PDF_DIR      turbo-html2pdf checkout (html2pdf-* suites; run in place)
#   HTML2PDF_TARGET_DIR     its target dir                          default $OUT/html2pdf-target
#   HTML2PDF_CORPUS_CHECKPOINT / HTML2PDF_CORPUS_EXPECT  as for surf-corpus (default b-main)
#   SURF_STATIC_HARNESS     surf-static harness package dir
#   CARGO_TARGET_DIR (turbo-dom), TT_TARGET_DIR, SURF_TARGET_DIR  target dirs (share them
#                    across the two modes to avoid cold rebuilds; the build id differs anyway)
#
# Steps (plan §7.3 "Driver"):
#   1. build with `--features selector-dual` (each repo gets its own TURBO_DOM_DUAL_BUILD_ID,
#      compiled into the turbo-dom it links); OUT/manifest.json records the build ids and the
#      sha256 of every built artifact (cargo --message-format=json for test binaries);
#   2. install: plain copy to a temp name + mv (never overwrite in place); OUT/install.tsv
#      records installed path -> sha256 measured after copying; originals are restored on exit;
#   3. run each suite directly with TURBO_DOM_SELECTOR_ENGINE=<mode> and its own
#      TURBO_DOM_SELECTOR_DUAL_LOG_DIR=OUT/logs/<suite> (never test:playwright / npm-build.sh,
#      which rebuild without the feature); cargo results -> OUT/<suite>.tsv;
#   4. scripts/selector-dual-check.py: every header's build id must be in the manifest, per-suite
#      vacuity guard (>= 1 header and >= 1 compared call), and every divergence classified
#      (Gate B; also reported in legacy mode). Gate A = compare OUT/*.tsv / json with the
#      previous step's reference results (docs/selector-dual-gate.md).
# Never pass --all-features to a dual-capable crate: features are always listed explicitly.
set -euo pipefail

step=${1:?usage: selector-dual-gate.sh <step> <legacy|new> [suite ...]}
mode=${2:?usage: selector-dual-gate.sh <step> <legacy|new> [suite ...]}
shift 2
case "$mode" in legacy | new) ;; *) echo "mode must be legacy or new" >&2; exit 2 ;; esac

root=$(cd "$(dirname "$0")/.." && pwd)
tt_src=${TURBO_TEST_DIR:-$root/../turbo-test-consolidation}
surf=${TURBO_SURF_DIR:-}
out=${OUT:-${TMPDIR:-/tmp}/selector-dual/$step-$mode}
tt_features=${TT_FEATURES:-turbo-dom-browser-env/selector-dual}
surf_features=${SURF_FEATURES:-selector-dual}
export TZ=UTC LANG=C CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_RELEASE_DEBUG=0
cargo=(cargo +1.98.0)
stamp=$(date +%s)

if [ "$#" -eq 0 ]; then
  set -- turbo-dom
  [ -d "$tt_src" ] && set -- "$@" turbo-test-cargo turbo-test-npm turbo-test-compat
  [ -n "$surf" ] && set -- "$@" surf-cargo surf-shim surf-e2e surf-detection surf-corpus
fi
suites=("$@")
wants() { local s; for s in "${suites[@]}"; do [ "$s" = "$1" ] && return 0; done; return 1; }
wants_prefix() { local s; for s in "${suites[@]}"; do case "$s" in "$1"*) return 0 ;; esac; done; return 1; }

mkdir -p "$out/logs"
manifest_ids=$out/build-ids.txt
artifacts=$out/artifacts.tsv
install_log=$out/install.tsv
: >"$manifest_ids"
: >"$artifacts"
: >"$install_log"

sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

# cargo --message-format=json -> artifacts.tsv (repo, kind, path, sha256)
record_artifacts() {
  local repo=$1 json=$2
  python3 - "$repo" "$json" "$artifacts" <<'PY'
import hashlib, json, sys
repo, src, dst = sys.argv[1:4]
with open(dst, "a") as out:
    for line in open(src):
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-artifact":
            continue
        paths = [msg["executable"]] if msg.get("executable") else [
            f for f in msg.get("filenames", []) if f.endswith((".dylib", ".so", ".node"))]
        for p in paths:
            digest = hashlib.sha256(open(p, "rb").read()).hexdigest()
            kind = "test" if msg.get("profile", {}).get("test") else msg["target"]["kind"][0]
            out.write(f"{repo}\t{kind}\t{p}\t{digest}\n")
PY
}

# results of a `cargo test` log -> sorted binary<TAB>test<TAB>result
cargo_results() {
  python3 - "$1" <<'PY'
import re, sys
running = re.compile(r"^\s*Running (?:unittests )?(\S+)(?: \((.+)\))?")
doctests = re.compile(r"^\s*Doc-tests (\S+)")
case = re.compile(r"^test (.+?) \.\.\. (ok|FAILED|ignored(?:, .*)?)$")
current, rows = "?", set()
for line in open(sys.argv[1], errors="replace"):
    line = line.rstrip("\n")
    m = running.match(line)
    if m:
        path = m.group(2) or m.group(1)
        name = re.sub(r"-[0-9a-f]{16}(\.exe)?$", "", path.rsplit("/", 1)[-1])
        current = f"{name}<{m.group(1)}>" if m.group(2) else name
        continue
    m = doctests.match(line)
    if m:
        current = f"doctest:{m.group(1)}"
        continue
    m = case.match(line)
    if m:
        rows.add((current, m.group(1), "ignored" if m.group(2).startswith("ignored") else m.group(2)))
for row in sorted(rows):
    print("\t".join(row))
PY
}

# install SRC at DST: temp copy + mv, original kept for restore, sha256 after copying
backups=()
install_file() {
  local src=$1 dst=$2
  mkdir -p "$(dirname "$dst")"
  if [ -e "$dst" ] && [ ! -e "$dst.selector-dual-orig" ]; then
    mv "$dst" "$dst.selector-dual-orig"
    backups+=("$dst")
  fi
  cp "$src" "$dst.selector-dual-tmp.$$"
  mv "$dst.selector-dual-tmp.$$" "$dst"
  printf '%s\t%s\n' "$dst" "$(sha "$dst")" >>"$install_log"
}
restore() {
  local dst
  for dst in "${backups[@]+"${backups[@]}"}"; do
    [ -e "$dst.selector-dual-orig" ] && mv "$dst.selector-dual-orig" "$dst"
  done
}
trap restore EXIT

suite_env() { # suite name -> env assignments for the run
  printf 'TURBO_DOM_SELECTOR_ENGINE=%s TURBO_DOM_SELECTOR_DUAL_LOG_DIR=%s' "$mode" "$out/logs/$1"
}

run_cargo_suite() { # suite dir cargo-args...
  local suite=$1 dir=$2
  shift 2
  echo "== $suite: cargo test $*"
  rm -rf "$out/logs/$suite"
  (cd "$dir" && env $(suite_env "$suite") "${cargo[@]}" test --no-fail-fast "$@") >"$out/$suite.log" 2>&1 || true
  cargo_results "$out/$suite.log" >"$out/$suite.tsv"
  echo "   $(cut -f3 "$out/$suite.tsv" | sort | uniq -c | tr '\n' ' ')"
}

# ------------------------------------------------------------------ 1. turbo-dom
if wants turbo-dom; then
  id="$step-$mode-turbo-dom-$stamp"
  echo "$id" >>"$manifest_ids"
  dom_args=(-p turbo-dom -p turbo-dom-browser-env --features
    turbo-dom/selector-dual,turbo-dom-browser-env/selector-dual,turbo-dom-browser-env/host-internals,turbo-dom-browser-env/test-support)
  echo "== build turbo-dom ($id)"
  (cd "$root" && TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" test --no-run --message-format=json "${dom_args[@]}") \
    >"$out/build-turbo-dom.json" 2>"$out/build-turbo-dom.err"
  record_artifacts turbo-dom "$out/build-turbo-dom.json"
  TURBO_DOM_DUAL_BUILD_ID=$id run_cargo_suite turbo-dom "$root" "${dom_args[@]}"
fi

# ------------------------------------------------------------------ 2. turbo-test (scratch copy)
if wants_prefix turbo-test; then
  tt=$out/turbo-test
  id="$step-$mode-turbo-test-$stamp"
  echo "$id" >>"$manifest_ids"
  echo "== copy $tt_src -> $tt (turbo-dom + browser-env patched to $root)"
  rm -rf "$tt"
  mkdir -p "$tt"
  # anchored excludes: fixtures carry their own node_modules (e.g. fixtures/jestdom-reuse)
  rsync -a --exclude=/target --exclude=/node_modules --exclude=/.git "$tt_src/" "$tt/"
  [ -d "$tt_src/node_modules" ] && ln -s "$tt_src/node_modules" "$tt/node_modules"
  python3 - "$tt/Cargo.toml" "$root" <<'PY'
import re, sys
path, root = sys.argv[1:3]
s = open(path).read()
head, sep, patch = s.partition("[patch.crates-io]")
if not sep:
    sys.exit(f"no [patch.crates-io] in {path}")
for name in ("turbo-dom", "turbo-dom-browser-env"):
    patch, n = re.subn(rf'^{re.escape(name)} = \{{[^\n]*\}}$', f'{name} = {{ path = "{root}/crates/{name}" }}', patch, flags=re.M)
    if n != 1:
        sys.exit(f"cannot patch {name} in {path}")
open(path, "w").write(head + sep + patch)
PY
  tt_target=${TT_TARGET_DIR:-$out/turbo-test-target}
  (cd "$tt" && CARGO_TARGET_DIR=$tt_target "${cargo[@]}" update -p turbo-dom-browser-env -p turbo-dom >"$out/turbo-test-relock.log" 2>&1)
  echo "== build turbo-test ($id)"
  (cd "$tt" && CARGO_TARGET_DIR=$tt_target TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" test --locked --no-run --message-format=json --features "$tt_features") \
    >"$out/build-turbo-test.json" 2>"$out/build-turbo-test.err"
  record_artifacts turbo-test "$out/build-turbo-test.json"
  if wants turbo-test-npm || wants turbo-test-compat; then
    (cd "$tt" && CARGO_TARGET_DIR=$tt_target TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" build --release --locked --message-format=json --features "$tt_features" --bin turbo-test) \
      >"$out/build-turbo-test-release.json" 2>"$out/build-turbo-test-release.err"
    record_artifacts turbo-test "$out/build-turbo-test-release.json"
    # cli.js prefers bin/turbo-test-<platform>; scripts/conformity.mjs uses target/release
    install_file "$tt_target/release/turbo-test" "$tt/bin/turbo-test-darwin-arm64"
    install_file "$tt_target/release/turbo-test" "$tt/target/release/turbo-test"
  fi
  if wants turbo-test-cargo; then
    CARGO_TARGET_DIR=$tt_target TURBO_DOM_DUAL_BUILD_ID=$id run_cargo_suite turbo-test-cargo "$tt" --locked --features "$tt_features"
  fi
  if wants turbo-test-npm; then
    echo "== turbo-test-npm"
    rm -rf "$out/logs/turbo-test-npm"
    (cd "$tt" && env $(suite_env turbo-test-npm) node --test 'test/*.mjs') >"$out/turbo-test-npm.log" 2>&1 || true
    grep -E '^ℹ (tests|pass|fail|skipped)' "$out/turbo-test-npm.log" | tr '\n' ' '; echo
  fi
  if wants turbo-test-compat; then
    echo "== turbo-test-compat"
    rm -rf "$out/logs/turbo-test-compat"
    (cd "$tt" && env $(suite_env turbo-test-compat) node cli.js --reporter json fixtures/compat/*.test.ts) \
      >"$out/turbo-test-compat.json" 2>"$out/turbo-test-compat.err" || true
    tail -1 "$out/turbo-test-compat.err"
  fi
fi

# ------------------------------------------------------------------ 3. turbo-surf (in place, restored)
if wants surf-cargo || wants surf-shim || wants surf-e2e || wants surf-detection || wants surf-corpus; then
  [ -n "$surf" ] || { echo "TURBO_SURF_DIR is not set" >&2; exit 2; }
  id="$step-$mode-turbo-surf-$stamp"
  echo "$id" >>"$manifest_ids"
  surf_target=${SURF_TARGET_DIR:-$out/surf-target}
  surf_env=(CARGO_TARGET_DIR=$surf_target TURBO_DOM_DUAL_BUILD_ID=$id)
  [ -n "${RUSTY_V8_ARCHIVE_SURF:-}" ] && surf_env+=(RUSTY_V8_ARCHIVE=$RUSTY_V8_ARCHIVE_SURF)
  echo "== build turbo-surf ($id)"
  (cd "$surf/rust" && env "${surf_env[@]}" "${cargo[@]}" test --workspace --no-run --message-format=json --features "$surf_features") \
    >"$out/build-surf.json" 2>"$out/build-surf.err"
  record_artifacts turbo-surf "$out/build-surf.json"
  napi_package=${SURF_NAPI_PACKAGE:-turbo-surf-napi}
  (cd "$surf/rust" && env "${surf_env[@]}" "${cargo[@]}" build --release --message-format=json -p "$napi_package" --features "$surf_features") \
    >"$out/build-surf-napi.json" 2>"$out/build-surf-napi.err"
  record_artifacts turbo-surf "$out/build-surf-napi.json"
  (cd "$surf/rust" && env "${surf_env[@]}" "${cargo[@]}" build --release --message-format=json -p "${SURF_DETECTION_PACKAGE:-turbo-surf-mcp}" --example detection_trace --features "$surf_features") \
    >"$out/build-surf-detection.json" 2>"$out/build-surf-detection.err"
  record_artifacts turbo-surf "$out/build-surf-detection.json"
  napi_lib=$(ls "$surf_target"/release/lib*napi*.dylib 2>/dev/null | head -1)
  napi_node=${SURF_NAPI_NODE:-$out/surf-napi/turbo-surf.node}
  # plain cp keeps the linker's ad-hoc signature (no re-signing)
  [ -n "$napi_lib" ] && install_file "$napi_lib" "$napi_node"
  detection=$out/surf-detection/detection_trace
  install_file "$surf_target/release/examples/detection_trace" "$detection"
  export TURBO_SURF_NAPI_PATH=$napi_node TURBO_SURF_DETECTION_BINARY=$detection
  if wants surf-cargo; then
    (export "${surf_env[@]}"; run_cargo_suite surf-cargo "$surf/rust" --workspace --features "$surf_features")
  fi
  for s in shim e2e detection; do
    wants "surf-$s" || continue
    echo "== surf-$s"
    rm -rf "$out/logs/surf-$s"
    (cd "$surf" && env $(suite_env "surf-$s") npm run "test:$s") >"$out/surf-$s.log" 2>&1 || true
    tail -3 "$out/surf-$s.log"
  done
  if wants surf-corpus; then
    # the live corpus renders every fixture through the installed dual addon; Gate A wants
    # it byte-equal to the checkpoint, Gate B to the plain-build (new engine) outputs
    echo "== surf-corpus"
    rm -rf "$out/logs/surf-corpus" "$out/surf-corpus"
    expect=()
    for f in ${SURF_CORPUS_EXPECT:-}; do expect+=(--expect "$f"); done
    env $(suite_env surf-corpus) python3 "$root/tools/parity/parity.py" run "$out/surf-corpus" --surf "$napi_node" \
      >"$out/surf-corpus.log" 2>&1 || true
    python3 "$root/tools/parity/parity.py" compare "$out/surf-corpus" "${SURF_CORPUS_CHECKPOINT:-b2-surf}" \
      "${expect[@]+"${expect[@]}"}" >"$out/surf-corpus-compare.log" 2>&1 || true
    tail -3 "$out/surf-corpus-compare.log"
  fi
fi

# ------------------------------------------------------------------ 4. S3b: layout + raster
layout_features() { # suite -> cargo feature args
  case $1 in
    layout) echo "--features turbo-dom-layout/selector-dual,turbo-dom-raster/selector-dual" ;;
    layout-svg) echo "--features turbo-dom-layout/selector-dual,turbo-dom-layout/svg" ;;
    layout-nodefault) echo "--no-default-features --features turbo-dom-layout/selector-dual" ;;
    layout-allfeat) echo "--features turbo-dom-layout/selector-dual,turbo-dom-layout/svg,turbo-dom-layout/perf,turbo-dom-layout/test-support" ;;
  esac
}
for suite in layout layout-svg layout-nodefault layout-allfeat; do
  wants "$suite" || continue
  id="$step-$mode-$suite-$stamp"
  echo "$id" >>"$manifest_ids"
  packages=(-p turbo-dom-layout)
  [ "$suite" = layout ] && packages+=(-p turbo-dom-raster)
  # shellcheck disable=SC2207
  feats=($(layout_features "$suite"))
  echo "== build $suite ($id)"
  (cd "$root" && TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" test --no-run --message-format=json "${packages[@]}" "${feats[@]}") \
    >"$out/build-$suite.json" 2>"$out/build-$suite.err"
  record_artifacts turbo-dom "$out/build-$suite.json"
  TURBO_DOM_DUAL_BUILD_ID=$id run_cargo_suite "$suite" "$root" "${packages[@]}" "${feats[@]}"
done

# ------------------------------------------------------------------ 5. S3b: turbo-html2pdf (in place)
h2p=${TURBO_HTML2PDF_DIR:-}
h2p_target=${HTML2PDF_TARGET_DIR:-$out/html2pdf-target}
h2p_suite_args() { # suite -> cargo args
  case $1 in
    html2pdf-ws) echo "--workspace --features turbo-html2pdf-core/selector-dual" ;;
    html2pdf-core-allfeat) echo "-p turbo-html2pdf-core --features selector-dual,bundled-fonts,perf,endnotes,print-color,append,stamp,svg,xref,pdf-a,pdf-ua,encrypt" ;;
    html2pdf-core-nodefault) echo "-p turbo-html2pdf-core --no-default-features --features selector-dual" ;;
    html2pdf-core-svg) echo "-p turbo-html2pdf-core --features selector-dual,svg" ;;
  esac
}
for suite in html2pdf-ws html2pdf-core-allfeat html2pdf-core-nodefault html2pdf-core-svg; do
  wants "$suite" || continue
  [ -n "$h2p" ] || { echo "TURBO_HTML2PDF_DIR is not set" >&2; exit 2; }
  id="$step-$mode-$suite-$stamp"
  echo "$id" >>"$manifest_ids"
  # shellcheck disable=SC2207
  args=($(h2p_suite_args "$suite"))
  echo "== build $suite ($id)"
  (cd "$h2p" && CARGO_TARGET_DIR=$h2p_target TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" test --no-run --message-format=json "${args[@]}") \
    >"$out/build-$suite.json" 2>"$out/build-$suite.err"
  record_artifacts turbo-html2pdf "$out/build-$suite.json"
  CARGO_TARGET_DIR=$h2p_target TURBO_DOM_DUAL_BUILD_ID=$id run_cargo_suite "$suite" "$h2p" "${args[@]}"
done
if wants html2pdf-corpus; then
  [ -n "$h2p" ] || { echo "TURBO_HTML2PDF_DIR is not set" >&2; exit 2; }
  id="$step-$mode-html2pdf-corpus-$stamp"
  echo "$id" >>"$manifest_ids"
  nodes=()
  for variant in selector-dual svg,selector-dual; do
    name=html2pdf$([ "$variant" = selector-dual ] || echo -svg)
    echo "== build $name napi ($id, features $variant)"
    (cd "$h2p" && CARGO_TARGET_DIR=$h2p_target-napi-$name TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" build --release --message-format=json -p turbo-pdf-napi --features "$variant") \
      >"$out/build-$name-napi.json" 2>"$out/build-$name-napi.err"
    record_artifacts turbo-html2pdf "$out/build-$name-napi.json"
    install_file "$h2p_target-napi-$name/release/libturbo_pdf_napi.dylib" "$out/html2pdf-napi/$name.node"
    nodes+=("$out/html2pdf-napi/$name.node")
  done
  echo "== html2pdf-corpus"
  rm -rf "$out/logs/html2pdf-corpus" "$out/html2pdf-corpus"
  expect=()
  for f in ${HTML2PDF_CORPUS_EXPECT:-}; do expect+=(--expect "$f"); done
  env $(suite_env html2pdf-corpus) python3 "$root/tools/parity/parity.py" run "$out/html2pdf-corpus" \
    --html2pdf "${nodes[0]}" --html2pdf-svg "${nodes[1]}" >"$out/html2pdf-corpus.log" 2>&1 || true
  python3 "$root/tools/parity/parity.py" compare "$out/html2pdf-corpus" "${HTML2PDF_CORPUS_CHECKPOINT:-b-main}" \
    "${expect[@]+"${expect[@]}"}" >"$out/html2pdf-corpus-compare.log" 2>&1 || true
  tail -3 "$out/html2pdf-corpus-compare.log"
fi

# ------------------------------------------------------------------ 6. S3b: surf static screenshots
if wants surf-static; then
  harness=${SURF_STATIC_HARNESS:?SURF_STATIC_HARNESS is not set}
  id="$step-$mode-surf-static-$stamp"
  echo "$id" >>"$manifest_ids"
  echo "== build surf-static ($id)"
  (cd "$harness" && CARGO_TARGET_DIR=$out/surf-static-target TURBO_DOM_DUAL_BUILD_ID=$id "${cargo[@]}" build --release --message-format=json) \
    >"$out/build-surf-static.json" 2>"$out/build-surf-static.err"
  record_artifacts surf-static "$out/build-surf-static.json"
  rm -rf "$out/logs/surf-static"
  bin=$(python3 -c 'import json,sys
for line in open(sys.argv[1]):
    try: m = json.loads(line)
    except ValueError: continue
    if m.get("reason") == "compiler-artifact" and m.get("executable"): print(m["executable"])' "$out/build-surf-static.json" | tail -1)
  # the inputs of tools/parity's `surf` + `surf-extra` static screenshots
  parity=$root/tools/parity
  (env $(suite_env surf-static) "$bin" "$parity/.cache/turbo-surf/harness" "$parity/.cache/turbo-surf/rust/crates/turbo-surf-render" \
    "$parity/.cache/turbo-html2pdf/benches/conformance/fixtures" "$parity/inputs/image-matrix/pages" "$parity/inputs/extra") \
    >"$out/surf-static.log" 2>&1 || true
  tail -1 "$out/surf-static.log"
fi

# ------------------------------------------------------------------ 7. manifest + checks
python3 - "$out" "$step" "$mode" <<'PY'
import json, sys
out, step, mode = sys.argv[1:4]
ids = [l.strip() for l in open(f"{out}/build-ids.txt") if l.strip()]
arts = [dict(zip(("repo", "kind", "path", "sha256"), l.rstrip("\n").split("\t"))) for l in open(f"{out}/artifacts.tsv") if l.strip()]
inst = [dict(zip(("path", "sha256"), l.rstrip("\n").split("\t"))) for l in open(f"{out}/install.tsv") if l.strip()]
json.dump({"step": step, "mode": mode, "build_ids": ids, "artifacts": arts, "installed": inst},
          open(f"{out}/manifest.json", "w"), indent=1)
print(f"manifest: {len(ids)} build ids, {len(arts)} artifacts, {len(inst)} installs")
PY
check_args=()
for s in "${suites[@]}"; do check_args+=("$s=$out/logs/$s"); done
# turbo-test's own cargo tests (runner, interceptor, V8 teardown) make no selector call
# since the binding moved to turbo-dom-browser-env: Gate A compares their results, the
# vacuity guard cannot apply.
python3 "$root/scripts/selector-dual-check.py" --mode "$mode" --manifest "$out/manifest.json" --classify \
  --allow-vacuous turbo-test-cargo --report "$out/check.json" "${check_args[@]}"
