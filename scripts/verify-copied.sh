#!/bin/sh
# Verify the ONE-TIME MOVE of turbo-test's rtdom<->V8 browser binding into
# crates/turbo-dom-browser-env/src.
#
# This is a review aid, NOT a sync tool: it never writes into the repository and
# nothing re-copies turbo-test sources after the move. turbo-test deletes its
# copy when it switches to the `turbo-dom-browser-env` crate; from then on this
# crate is the only home of the binding and is edited here directly.
#
# Source of the move: miaskiewicz/turbo-test@cb5c64ee7cb6 (v0.4.8), `src/`
# minus the runner-only files listed in EXCLUDES below (398 files).
#
# Modes
#   verbatim [REV]  REV's crates/turbo-dom-browser-env/src must equal the source
#                   byte-for-byte with the identical file set. REV defaults to
#                   the commit whose message records
#                   "from miaskiewicz/turbo-test@cb5c64e".
#   current  [REV]  (default; REV defaults to HEAD) re-derives the moved files as
#                   rustfmt(rewrite(source)) and compares them with REV:
#                     - every moved file must exist in REV;
#                     - a moved file that differs must be listed in
#                       SEMANTIC_EDITS (its diff is printed for review);
#                     - every other file under src/ must be listed in OWNED
#                       (crate files that were never part of the move).
#
# Allow-list of mechanical rewrites (applied, not diffed):
#   1. scripts/browser-env-rewrite.sh: appends the crate-surface hook
#      (`#[path = "crate_surface.rs"] mod crate_surface; pub use crate_surface::*;`)
#      to browser_env.rs. It is the ONLY scripted rewrite.
#   2. rustfmt (edition 2021, toolchain $RUSTFMT_TOOLCHAIN, default 1.98.0),
#      i.e. the "cargo fmt" commit.
# Anything else that differs from the source is a semantic edit and must be in
# SEMANTIC_EDITS with the commit that explains it.
#
# Usage: scripts/verify-copied.sh [--turbo-test DIR] [verbatim|current] [REV]
#   DIR: a turbo-test clone containing cb5c64e (default: $TURBO_TEST_REPO, else
#   ../turbo-test next to this repository). Or set TURBO_TEST_FETCH=1 to fetch
#   the commit from https://github.com/miaskiewicz/turbo-test into a temp clone.
set -eu

SOURCE_SHA=cb5c64ee7cb678690679e9d9c58960f929178ee0
CRATE_SRC=crates/turbo-dom-browser-env/src
# turbo-test runner files that stay in turbo-test (paths relative to src/).
EXCLUDES="bin bundler.rs coverage.rs coverage_branch.rs esm_cjs.rs graph.rs launcher.rs napi_host.rs runner.rs transform.rs runtime.js lib.rs"
# Moved files edited semantically after the move (path: reason).
SEMANTIC_EDITS="
cssom_parser.rs: zoom_value + helpers moved to turbo_dom::rtdom::css_values (re-exported here)
interceptor_mode.rs: frame read placed behind the native-sdk-frame-read feature scaffolding
browser_env.rs: module docs name the turbo-dom crate and drop the retired TURBO_RUST_DOM gate; P5b turbo-dom#20 native functions take a call-without-new message, named definers pass V8's should-throw mode; P5b turbo-surf#32 installs html_image.js + its tests; P5b-X2 turbo-dom#24 clones keep the custom element state; P5b-X3 turbo-surf#36 computed-defaults control test module
custom_elements.js: P5b-X2 turbo-dom#24 reports custom element states to the tree (:defined)
custom_elements.rs: P5b-X2 turbo-dom#24 __customElementState native + the :defined Chrome 156 control test
document_basics.rs: P5b-X2 turbo-dom#24 adoption moves the custom element state with the node
css_admission_tests.rs: comment no longer names turbo-test as the standalone host; P5b-X3 turbo-surf#36 a rule-less visibility is the initial visible
css_selector_syntax.rs: doc comment no longer names turbo-test as the standalone host
editing_tests.rs: deviation notes say \"standalone binding\" instead of turbo-test
view_transitions_tests.rs: comment no longer calls the binding vendored
select_element.js: P5b turbo-dom#20 Option() legacy factory function; quiet accessor definitions on HTMLOptionsCollection
select_element_tests.rs: P5b turbo-dom#20 Option() factory + options-collection definer vs their Chrome 156 control
browser_env.js: P5b-X2 turbo-dom#23 drops the JS Image() fallback (own src + fake onload); Image() is native in html_image.js; P5b-X3 turbo-surf#36 getComputedStyle UA display / position / visibility defaults, no eager <title>
document_basics.js: P5b turbo-dom#20 live collections pass the definer's should-throw mode to setIndex
html_popover.rs: P5b turbo-dom#18 internal selector paths anchor with :not(* > *) instead of :root (6a23551)
dom_implementation_tests.rs: P5b the dom-implementation collector must label its control like the committed one
"
# Crate-owned files under src/ that were not part of the move.
OWNED="lib.rs crate_surface.rs host_internals.rs test_support.rs css_snapshot.rs option_factory_probe.js option_factory_cases.json option_factory_chrome156.json html_image.js html_image_tests.rs html_image_probe.js html_image_cases.json html_image_chrome156.json custom_elements_defined_probe.js custom_elements_defined_cases.json custom_elements_defined_chrome156.json computed_defaults_probe.js computed_defaults_cases.json computed_defaults_chrome156.json computed_defaults_tests.rs"

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
turbo_test=${TURBO_TEST_REPO:-$repo_root/../turbo-test}
if [ "${1:-}" = "--turbo-test" ]; then
  turbo_test=$2
  shift 2
fi
mode=${1:-current}
rev=${2:-}

work=$(mktemp -d "${TMPDIR:-/tmp}/verify-copied.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

if [ "${TURBO_TEST_FETCH:-0}" = 1 ]; then
  turbo_test=$work/turbo-test.git
  git init -q --bare "$turbo_test"
  git -C "$turbo_test" fetch -q --depth 1 https://github.com/miaskiewicz/turbo-test "$SOURCE_SHA"
fi
git -C "$turbo_test" cat-file -e "$SOURCE_SHA^{commit}" 2>/dev/null || {
  echo "turbo-test commit $SOURCE_SHA not found in $turbo_test" >&2
  echo "(pass --turbo-test DIR, set TURBO_TEST_REPO, or TURBO_TEST_FETCH=1)" >&2
  exit 2
}

# 1. Re-derive the moved file set from the source commit.
mkdir -p "$work/source"
git -C "$turbo_test" archive "$SOURCE_SHA" src | tar -x -C "$work/source"
for path in $EXCLUDES; do
  rm -rf "$work/source/src/$path"
done
(cd "$work/source/src" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) >"$work/moved.list"
echo "source: miaskiewicz/turbo-test@$SOURCE_SHA, $(wc -l <"$work/moved.list" | tr -d ' ') moved files"

extract_rev() { # $1 = rev, $2 = destination
  mkdir -p "$2"
  git -C "$repo_root" archive "$1" "$CRATE_SRC" | tar -x -C "$2"
}

case $mode in
verbatim)
  if [ -z "$rev" ]; then
    rev=$(git -C "$repo_root" log --format=%H --grep='from miaskiewicz/turbo-test@cb5c64e' -- "$CRATE_SRC" | tail -n 1)
    [ -n "$rev" ] || { echo "no verbatim-copy commit found; pass REV" >&2; exit 2; }
  fi
  extract_rev "$rev" "$work/rev"
  if diff -r "$work/source/src" "$work/rev/$CRATE_SRC" >"$work/diff.txt"; then
    echo "OK: $CRATE_SRC at $(git -C "$repo_root" rev-parse --short "$rev") is a byte-for-byte copy"
  else
    cat "$work/diff.txt"
    echo "FAIL: $CRATE_SRC at $rev differs from the source" >&2
    exit 1
  fi
  ;;
current)
  rev=${rev:-HEAD}
  extract_rev "$rev" "$work/rev"
  derived=$work/source/src
  sh "$repo_root/scripts/browser-env-rewrite.sh" "$derived"
  # rustfmt follows `mod` declarations, so the crate-owned modules must be present;
  # they are excluded from the comparison below.
  for path in $OWNED; do
    if [ -f "$work/rev/$CRATE_SRC/$path" ] && [ "$path" != lib.rs ]; then
      cp "$work/rev/$CRATE_SRC/$path" "$derived/$path"
    fi
  done
  rustfmt "+${RUSTFMT_TOOLCHAIN:-1.98.0}" --edition 2021 "$derived/browser_env.rs"
  status=0
  semantic=0
  while IFS= read -r path; do
    target=$work/rev/$CRATE_SRC/$path
    if [ ! -f "$target" ]; then
      echo "FAIL: moved file missing: $path" >&2
      status=1
    elif ! cmp -s "$derived/$path" "$target"; then
      if printf '%s\n' "$SEMANTIC_EDITS" | grep -q "^$path:"; then
        semantic=$((semantic + 1))
        echo "== semantic edit (allow-listed): $path"
        diff -u "$derived/$path" "$target" | sed "s|$work/[a-z]*/||" || true
      else
        echo "FAIL: $path differs from rustfmt(rewrite(source)) and is not in SEMANTIC_EDITS" >&2
        diff -u "$derived/$path" "$target" | head -n 40 >&2 || true
        status=1
      fi
    fi
  done <"$work/moved.list"
  (cd "$work/rev/$CRATE_SRC" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) >"$work/rev.list"
  for path in $(LC_ALL=C comm -13 "$work/moved.list" "$work/rev.list"); do
    case " $OWNED " in
    *" $path "*) ;;
    *)
      echo "FAIL: $path is neither moved nor listed in OWNED" >&2
      status=1
      ;;
    esac
  done
  if [ "$status" = 0 ]; then
    echo "OK: $(wc -l <"$work/moved.list" | tr -d ' ') moved files = rustfmt(rewrite(source)) except $semantic allow-listed semantic edit(s)"
  fi
  exit "$status"
  ;;
*)
  echo "unknown mode: $mode (verbatim|current)" >&2
  exit 2
  ;;
esac
