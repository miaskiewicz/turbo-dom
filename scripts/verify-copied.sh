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
browser_env.rs: module docs name the turbo-dom crate and drop the retired TURBO_RUST_DOM gate; P5b turbo-dom#20 native functions take a call-without-new message, named definers pass V8's should-throw mode; P5b turbo-surf#32 installs html_image.js + its tests; P5b-X2 turbo-dom#24 clones keep the custom element state; P5b-X3 turbo-surf#36 computed-defaults control test module; P5b-X5 turbo-dom#43 setters are [CEReactions] scopes and fragment-parser setters report their parsed elements; P5b-X5 turbo-dom#44 validate_namespace_name splits strictly on ':'; P5b-X6 turbo-dom#66 shadow-events and turbo-dom#74 html-track control test modules; P5b-X6 turbo-dom#66 dispatchEvent builds the shadow-crossing event path; P5b-X6 turbo-dom#74 <track> wraps as HTMLTrackElement and installs html_track.js
custom_elements.js: P5b-X2 turbo-dom#24 reports custom element states to the tree (:defined); P5b-X5 turbo-dom#42/#43 queued upgrades, parsed-element upgrades, connected-only insertion upgrades, upgrade(root) per registry; P5b-X6 turbo-dom#74 constructors keep their interface constants and throw Chrome's Failed to construct message
custom_elements.rs: P5b-X2 turbo-dom#24 __customElementState native + the :defined Chrome 156 control test; P5b-X5 turbo-dom#43 parser_inserted / event_mark (parsed elements' upgrades ahead of a setter's removals)
document_basics.rs: P5b-X2 turbo-dom#24 adoption moves the custom element state with the node
css_admission_tests.rs: comment no longer names turbo-test as the standalone host; P5b-X3 turbo-surf#36 a rule-less visibility is the initial visible
css_selector_syntax.rs: doc comment no longer names turbo-test as the standalone host
editing_tests.rs: deviation notes say \"standalone binding\" instead of turbo-test
view_transitions_tests.rs: comment no longer calls the binding vendored
select_element.js: P5b turbo-dom#20 Option() legacy factory function; quiet accessor definitions on HTMLOptionsCollection
select_element_tests.rs: P5b turbo-dom#20 Option() factory + options-collection definer vs their Chrome 156 control
browser_env.js: P5b-X2 turbo-dom#23 drops the JS Image() fallback (own src + fake onload); Image() is native in html_image.js; P5b-X3 turbo-surf#36 getComputedStyle UA display / position / visibility defaults, no eager <title>; P5b-X5 turbo-dom#30 dialog:modal / top layer / ::backdrop, turbo-dom#47 pseudoElt parsing and ::before / ::after; P5b-X6 turbo-dom#61 form control UA fonts, turbo-dom#47 UA-styled pseudo-elements, turbo-dom#58 fullscreen UA values, viewport-sized width / height and backdrop, turbo-dom#67 disconnected elements compute no style, turbo-dom#68 document sheets do not select shadow-tree elements, turbo-dom#66 relatedTarget retargeting through the event record, turbo-dom#74 TRACK in htmlByTag
document_basics.js: P5b turbo-dom#20 live collections pass the definer's should-throw mode to setIndex
html_popover.rs: P5b turbo-dom#18 internal selector paths anchor with :not(* > *) instead of :root (6a23551)
shadow_html.rs: P5b-X5 turbo-dom#43 setHTMLUnsafe queues the parsed elements' custom element upgrades before its removals; P5b-X6 turbo-dom#66 tree_root for the event path
shadow_slots.rs: P5b-X6 turbo-dom#66 event_slot (a slotted node's event-path parent)
reference_target_tests.rs: P5b-X6 turbo-dom#66 composed invoker events reach the host (the known gap is gone)
dom_implementation.js: P5b-X5 turbo-dom#57 createDocument validates and extracts its qualified name like createElementNS
mutation_observers.rs: P5b-X5 turbo-dom#54 flush delivers an independently installed document's recorded mutations when the call returns
dom_implementation_tests.rs: P5b the dom-implementation collector must label its control like the committed one
html_dialog.rs: P5b-X6 turbo-dom#58 :fullscreen matches in the selector engine (no rewrite); :modal also matches the fullscreen element
user_activation_gates.rs: P5b-X6 turbo-dom#58 the fullscreen stack is reported to the tree (Tree::set_fullscreen_stack)
browser_env.js: P5b-X7 turbo-surf#42/turbo-dom#72 ts.DOMException / ts.realmDOMException (the realm's registered DOMException, resolved when raised) for every site; turbo-dom#73 reflected src/href getters through ts.resolveDocumentURL
browser_env.rs: P5b-X7 __nativeRealmDOMException (turbo-surf#42) and __nativeResolveDocumentURL (turbo-dom#71/#73) natives; test modules dom_exception_realm_tests, custom_elements_collect_tests, html_image_srcset_tests, reflected_urls_tests
event_listeners.rs: P5b-X7 turbo-surf#42 capture_window_event_interfaces registers the host's installed DOMException
custom_elements.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException
dom_implementation.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException
document_basics.js: P5b-X7 turbo-dom#72 exceptions from ts.DOMException
event_listeners.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException (the capture hook no longer swaps it)
range_contents.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException (the capture hook no longer swaps it)
animations.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
document_legacy.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
document_xpath.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
edit_context.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
element_internals.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
file_inputs.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
form_controls.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
html_dialog.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
html_text.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
input_members.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
pointer_events.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
selection.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
typed_om.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
user_activation_gates.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install
editing.js: P5b-X7 turbo-dom#72 exceptions from ts.DOMException
element_tokens.js: P5b-X7 turbo-dom#72 exceptions from ts.DOMException
browser_env.js: P5b-X8 turbo-dom#87 MathMLElement interface; style / autofocus / focus() / blur() / on* on SVGElement and MathMLElement with per-interface brand checks; ts.focusElement for internal focusing steps
browser_env.rs: P5b-X8 turbo-dom#87 SVGElement / MathMLElement accessor brands; svg_mathml_element control test module
document_basics.js: P5b-X8 turbo-dom#87 dataset / nonce / tabIndex on HTMLElement, SVGElement and MathMLElement, each brand-checked
element_tokens.js: P5b-X8 turbo-dom#87 focusGroup on MathMLElement
focus_group.js: P5b-X8 turbo-dom#87 focusGroupStart on MathMLElement; navigation focuses through ts.focusElement
focus_group_tests.rs: P5b-X8 turbo-dom#87 the MathMLElement known gap is gone
html_dialog.js: P5b-X8 turbo-dom#87 dialog focusing steps focus through ts.focusElement
browser_env.js: P5b-X8 turbo-dom#90 CSSStyleDeclaration expands / serializes shorthands with rtdom::css_shorthand (__cssShorthandNative), cssText parsing moves repeated properties to the end, longhand aliases resolve; the computed cascade expands UA / animation shorthands natively and drops shorthand entries a later longhand outdates
browser_env.rs: P5b-X8 turbo-dom#90 registers __cssShorthandNative (css_shorthand.rs); css_shorthand control test module
browser_env.js: P5b-X8 turbo-dom#97 longhands take rtdom::css_shorthand::longhand_value spelling / admission; the computed cascade reads the stylesheet grammar text
browser_env.rs: P5b-X8 turbo-dom#97 css_longhand control test module
key_order_tests.rs: P5b-X8 turbo-dom#99 deferred key order moves HTMLTrackElement constants installed before their attributes
browser_env.js: P5b-H1 turbo-dom#110/#111 the 2D context's save() / restore() keep a drawing-state stack, setLineDash stores the dash, the initial direction is inherit
key_order_tests.rs: P5b-H1 turbo-dom#99 a second install into an arranged realm keeps HTMLTrackElement's key order
css_live_tests.rs: P5b-K1 turbo-test#125 getComputedStyle reuse invalidation tests (tree version, CSSOM epoch, top layer, font context, ancestor styles); P5b-L1 turbo-dom#113 cross-realm CSSOM edits
dom_ownership.rs: P5b-K3 turbo-dom#112 deferred ownership reconcile above 256 wrappers + flush_ownership, GC tracking in install, reconcile_all in its own handle scope (from turbo-test PR #105); turbo-test#127 pin_callback_owners without per-call allocation; test modules dom_ownership_alloc_tests, dom_ownership_scale_tests
document_basics.rs: P5-cov-browser-env turbo-dom#137 adoptNode(shadowRoot) throws HierarchyRequestError
view_transitions.rs: P5-cov-browser-env turbo-dom#136 :active-view-transition-type() admits keyword identifiers
browser_env.rs: P5-cov-browser-env removes the element template's unreachable constructor / dataset getters and containsNode method (wrappers are projected before page script runs and JS owns those members: the interface prototypes, document_basics.js dataset, selection.js containsNode)
event_brands.rs: P5-cov-browser-env literal v8::String::new keys unwrap (a short literal cannot exceed v8::String::kMaxLength, so the else-return arms were unreachable)
range_contents.rs: P5-cov-browser-env literal v8::String::new keys unwrap (a short literal cannot exceed v8::String::kMaxLength, so the else-return arms were unreachable)
reference_target.rs: P5-cov-browser-env literal v8::String::new keys unwrap (a short literal cannot exceed v8::String::kMaxLength, so the else-return arms were unreachable)
shadow_html.rs: P5-cov-browser-env literal v8::String::new keys unwrap (a short literal cannot exceed v8::String::kMaxLength, so the else-return arms were unreachable)
shadow_slots.rs: P5-cov-browser-env literal v8::String::new keys unwrap (a short literal cannot exceed v8::String::kMaxLength, so the else-return arms were unreachable)
animations_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
control_interfaces_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
css_admission_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
css_adoption_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
dom_implementation_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
dom_projection_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
editing_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
file_inputs_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
form_outputs_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
form_pseudo_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
html_dialog_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
key_order_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
pointer_events_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
sanitizer_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
select_element_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
utf16_contract_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
view_transitions_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
probe_harness_tests.rs: P5-cov-browser-env harness failure messages via crate_surface::harness_tests::thrown on the line that runs; compare helpers return their differences and have their own tests
shadow_slots.rs: P5-cov-browser-env native receiver / argument guards the JS caller makes unreachable become expect() (brand-checked receivers, the arguments object, converted markup; the last operation is the catch-all)
shadow_html.rs: P5-cov-browser-env native receiver / argument guards the JS caller makes unreachable become expect() (brand-checked receivers, the arguments object, converted markup; the last operation is the catch-all)
browser_env.rs: P5-cov-browser-env removes the unreachable native CharacterData data methods (document_basics.js owns them) and the item / namedItem methods bound on raw node arrays (every public collection copies the rows into its own live collection)
browser_env.js: P5-cov-browser-env no longer copies the removed native CharacterData data methods onto CharacterData.prototype
browser_env.rs: P5-cov-browser-env element-template accessors read their holder's handle through holder_handle (template instances always carry one); the unreachable DocumentType publicId / systemId / __nodeName template getters are gone (document_basics.js defines DocumentType)
browser_env.js: P5-cov-browser-env drops the DocumentType accessors of the replaced interface (document_basics.js defines DocumentType)
browser_env.rs: P5-cov-browser-env drops the template's checkValidity / reportValidity natives and contentEditable setter (form_controls.js and editing.js own them); nodeName's string fallback names only the constant-named kinds
browser_env.js: P5-cov-browser-env stops copying the removed checkValidity / reportValidity natives and the contentEditable setter
browser_env.rs: P5-cov-browser-env attachShadow's native states the ShadowRootInit its custom_elements.js wrapper validated (receiver, mode, slotAssignment) instead of re-validating it
browser_env.rs: P5-cov-browser-env the node addEventListener / removeEventListener / dispatchEvent natives state that their callers (browser_env.js's EventTarget wrappers, dispatch_host_event) pass a node and enough arguments
sanitizer.rs: P5-cov-browser-env natives called only by their JS shim drop the termination check (script cannot run while terminating) and state the shim's guarantees with expect() (object receivers, converted markup, canonical records, private records the module writes, wrapper creation contexts); the last named operation is the catch-all arm
shadow_slots.rs: P5-cov-browser-env natives called only by their JS shim drop the termination check (script cannot run while terminating) and state the shim's guarantees with expect() (object receivers, converted markup, canonical records, private records the module writes, wrapper creation contexts); the last named operation is the catch-all arm
shadow_html.rs: P5-cov-browser-env natives called only by their JS shim drop the termination check (script cannot run while terminating) and state the shim's guarantees with expect() (object receivers, converted markup, canonical records, private records the module writes, wrapper creation contexts); the last named operation is the catch-all arm
template_contents.rs: P5-cov-browser-env template walks state their guarantees with expect() (wrapped nodes, the installed document, importable template children); serialize drops the doctype / document / fragment arms no queued node can take
event_listeners.js: P5-cov-browser-env turbo-dom#156 addEventListener's {signal: null} is a TypeError (its option-order test's signal getter returns undefined)
event_listeners.rs: P5-cov-browser-env turbo-dom#156 addEventListener's {signal: null} is a TypeError (its option-order test's signal getter returns undefined)
document_basics.rs: P5-cov-browser-env the dispatch native's converted arguments (uint() offsets / counts, string() data / targets / nonces) are expect()s
"
# Crate-owned files under src/ that were not part of the move.
OWNED="lib.rs crate_surface.rs host_internals.rs test_support.rs css_snapshot.rs option_factory_probe.js option_factory_cases.json option_factory_chrome156.json html_image.js html_image_tests.rs html_image_probe.js html_image_cases.json html_image_chrome156.json custom_elements_defined_probe.js custom_elements_defined_cases.json custom_elements_defined_chrome156.json computed_defaults_probe.js computed_defaults_cases.json computed_defaults_chrome156.json computed_defaults_tests.rs dom_reactions_names_probe.js dom_reactions_names_cases.json dom_reactions_names_chrome156.json dom_reactions_tests.rs dom_adoption_reactions_probe.js dom_adoption_reactions_cases.json dom_adoption_reactions_chrome156.json shadow_events_probe.js shadow_events_cases.json shadow_events_chrome156.json shadow_events_tests.rs html_track_probe.js html_track_cases.json html_track_chrome156.json html_track_tests.rs html_track.js"
OWNED="$OWNED canvas_style_probe.js canvas_style_chrome156.json canvas_style_tests.rs canvas_state_probe.js canvas_state_chrome156.json"
OWNED="$OWNED css_shorthand.rs css_shorthand_tests.rs css_longhand_tests.rs svg_mathml_element_probe.js svg_mathml_element_cases.json svg_mathml_element_chrome156.json svg_mathml_element_tests.rs"
OWNED="$OWNED dom_exception_realm_probe.js dom_exception_realm_cases.json dom_exception_realm_chrome156.json dom_exception_realm_tests.rs custom_elements_collect_tests.rs html_image_srcset_probe.js html_image_srcset_cases.json html_image_srcset_chrome156.json html_image_srcset_tests.rs reflected_urls_probe.js reflected_urls_cases.json reflected_urls_chrome156.json reflected_urls_tests.rs"
OWNED="$OWNED dom_ownership_alloc_tests.rs dom_ownership_scale_tests.rs"
OWNED="$OWNED native_guards_probe.js native_guards_chrome156.json native_guards_tests.rs close_watchers_probe.js close_watchers_chrome156.json close_watchers_tests.rs host_api_tests.rs"
OWNED="$OWNED edge_probes_tests.rs harness_tests.rs termination_tests.rs shadow_slots_edges_probe.js shadow_slots_edges_chrome156.json dom_mutation_validity_probe.js dom_mutation_validity_chrome156.json selector_edges_probe.js selector_edges_chrome156.json sanitizer_edges_probe.js sanitizer_edges_chrome156.json document_edges_probe.js document_edges_chrome156.json receivers_edges_probe.js receivers_edges_chrome156.json event_receivers_edges_probe.js event_receivers_edges_chrome156.json"

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
