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
dom_implementation.rs: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
xpath_provider/README.md: 0.7.0 turbo-dom#185 documents fragment/shadow-root contexts and the empty namespace axis
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
document_xpath.js: P5b-X7 turbo-surf#42 exceptions from ts.DOMException instead of a constructor captured at install; 0.7.0 turbo-dom#185 fragment/shadow-root contexts, topmost-ancestor root, empty namespace axis
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
user_activation_gates.js: P5-cov-browser-env turbo-dom#129/#130 a fullscreen request's change task unfocuses outside the element just before firing, and targets the document once the element left it
user_activation_gates.rs: P5-cov-browser-env the hook dispatcher's unreachable guards become expects with their proof (the JS hook passes nodes and events, never asks the history-action queries, never sends dialog-opened / dialog-closed); the dialog light dismiss reads the topmost open dialog once; sweep_storage unit test
user_activation_gates.js: P5-cov-browser-env the query constants are the native's QUERIES indices (0-3)
html_dialog.rs: P5-cov-browser-env the hook dispatcher's and the DOM-scope paths' unreachable guards become expects with their proof
html_popover.rs: P5-cov-browser-env the hook dispatcher's, flush's and the DOM-scope paths' unreachable guards become expects with their proof
custom_elements.rs: P5-cov-browser-env argument guards of the natives only custom_elements.js calls become expects with their proof
selection.rs: P5-cov-browser-env turbo-dom#145 a visibility:hidden <br> still breaks the line in the visible-text model
selection.rs: P5-cov-browser-env the hook dispatcher's unreachable guards (target, unknown operation) and the selection.js source allocation become expects with their proof
browser_env.rs: P5-cov-browser-env removes the unreachable native CharacterData data methods (document_basics.js owns them) and the item / namedItem methods bound on raw node arrays (every public collection copies the rows into its own live collection)
browser_env.js: P5-cov-browser-env no longer copies the removed native CharacterData data methods onto CharacterData.prototype
browser_env.rs: P5-cov-browser-env element-template accessors read their holder's handle through holder_handle (template instances always carry one); the unreachable DocumentType publicId / systemId / __nodeName template getters are gone (document_basics.js defines DocumentType)
browser_env.js: P5-cov-browser-env drops the DocumentType accessors of the replaced interface (document_basics.js defines DocumentType)
browser_env.rs: P5-cov-browser-env drops the template's checkValidity / reportValidity natives and contentEditable setter (form_controls.js and editing.js own them); nodeName's string fallback names only the constant-named kinds
browser_env.js: P5-cov-browser-env stops copying the removed checkValidity / reportValidity natives and the contentEditable setter
browser_env.rs: P5-cov-browser-env attachShadow's native states the ShadowRootInit object literal its custom_elements.js wrapper built and validated (mode, slotAssignment) instead of re-validating it
browser_env.rs: P5-cov turbo-dom#149 a listener ended by execution termination ends the dispatch (rethrown, no later listener or default action)
browser_env.rs: P5-cov turbo-dom#157 a stop set in the target's capture listeners also skips its bubble listeners (Chrome 156)
browser_env.rs: P5-cov-browser-env round 3: the remaining element-template accessors (mode, __attrValue, outerHTML, ownerDocument, isContentEditable) take holder_handle; hide_node_nonce states nonce_units' handle
browser_env.rs: P5-cov-browser-env drops the template isContentEditable getter (editing.js defines it and only brand-checked through the old getter, discarding its value)
browser_env.rs: P5-cov turbo-dom#159 an invalid special form action canonicalizes its host (lowercase, unreserved escapes decoded, other escapes uppercase)
browser_env.rs: P5-cov-browser-env the inline style accessor takes holder_handle (an element-template accessor)
browser_env.rs: P5-cov-browser-env test-only termination checkpoints (cfg(test) watchdog; a no-op otherwise) in wrap_inner and the ownership method / setter guards
browser_env.js: P5-cov-browser-env no longer defines the replaced isContentEditable accessor (editing.js brand-checks through contentEditable)
event_listeners.rs: P5-cov turbo-dom#157 the stop contract test expects Chrome 156's target stop (native_guards_chrome156.json events.stopAtTarget)
sanitizer.rs: P5-cov-browser-env natives called only by their JS shim drop the termination check (script cannot run while terminating) and state the shim's guarantees with expect() (object receivers, converted markup, canonical records, private records the module writes, wrapper creation contexts); the last named operation is the catch-all arm
shadow_slots.rs: P5-cov-browser-env natives called only by their JS shim drop the termination check (script cannot run while terminating) and state the shim's guarantees with expect() (object receivers, converted markup, canonical records, private records the module writes, wrapper creation contexts); the last named operation is the catch-all arm
shadow_html.rs: P5-cov-browser-env natives called only by their JS shim drop the termination check (script cannot run while terminating) and state the shim's guarantees with expect() (object receivers, converted markup, canonical records, private records the module writes, wrapper creation contexts); the last named operation is the catch-all arm
template_contents.rs: P5-cov-browser-env template walks state their guarantees with expect() (wrapped nodes, the installed document, importable template children); serialize drops the doctype / document / fragment arms no queued node can take
event_listeners.js: P5-cov-browser-env turbo-dom#156 addEventListener's {signal: null} is a TypeError (its option-order test's signal getter returns undefined)
event_listeners.rs: P5-cov-browser-env turbo-dom#156 addEventListener's {signal: null} is a TypeError (its option-order test's signal getter returns undefined)
document_basics.rs: P5-cov-browser-env the dispatch native's converted arguments (uint() offsets / counts, string() data / targets / nonces) are expect()s
dom_mutations.rs: P5-cov-browser-env natives / mutation planning state their callers' guarantees with expect() (wrapped targets, an installed document, adopted planned nodes); the last named operation is the catch-all arm
view_transitions.rs: P5-cov-browser-env natives / mutation planning state their callers' guarantees with expect() (wrapped targets, an installed document, adopted planned nodes); the last named operation is the catch-all arm
user_activation_gates.rs: P5-cov-browser-env turbo-dom#150 each established close watcher has an identity; a request stops when its cancel action closed and reopened the dialog
html_dialog.rs: P5-cov-browser-env drops close_blocks (valid_selector already closes blocks left open at EOF; its every mutant survived the EOF selector control)
mutation_observers.rs: P5-cov-browser-env the observer native states mutation_observers.js's object arguments with expect() (schedule is the catch-all); a document wrapper's creation context is an expect()
document_legacy.rs: P5-cov-browser-env canonical_host's bracketed and ends-in-a-number parses map Ok(host) directly (the WHATWG parser yields only IPv6 / IPv4 there)
range_contents.rs: P5-cov-browser-env the contextual-fragment native states its caller's guarantees (a node start container, an installed document, fragment adoption) and install reads the ts namespace / makeNativeFunction browser_env.js defined, with expect()
event_handlers.rs: P5-cov-browser-env turbo-dom#152 an inline handler's form owner is found by HTML local name (tag_name is the uppercase tagName)
html_text.rs: P5-cov-browser-env turbo-dom#154 content of a closed <details> has an empty innerText (rendered with its contents skipped)
html_text.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
animations.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
document_xpath.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d); 0.7.0 turbo-dom#185 snapshots fragment/shadow-root/detached roots and doctypes
html_invokers.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
interest_invokers.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
pointer_events.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
element_internals.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
input_members.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
hyperlink_utils.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
editing.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
focus_group.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
reference_target.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
element_members.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
file_inputs.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
event_handlers.rs: P5-cov-browser-env native argument / receiver guards the JS caller makes unreachable become expect() and the last operation it calls is the match's wildcard arm (C3d)
script_state.rs: P5-cov-browser-env (C3d) the text setter's Trusted Types conversion binds its result before the let-else
script_state.rs: P5-cov-browser-env (C3d) copy takes the cloneNode / importNode source and clone nodes (their handles always exist)
input_members.rs: P5-cov-browser-env (C3d) copy takes the cloneNode / importNode source and clone nodes (their handles always exist)
editing_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
file_inputs_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
computed_defaults_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
typed_om_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
animations_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
probe_harness_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
document_legacy_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
ownership_guard_tests.rs: P5-cov-browser-env (C3d) harness failure branches run on their passing line (eager messages, unwrap_or_else(thrown), one drain checkpoint after the task loop); stand-in hooks and compare helpers have their own tests
html_popover.rs: P5-cov-browser-env (C3e) turbo-dom#160 forgiving :is()/:where() lists drop an invalid :popover-open() branch
view_transitions.rs: P5-cov-browser-env (C3e) turbo-dom#161 MathML roots get the UA math / block math box, unsupported internal ruby displays are invalid, only SVG-namespace <svg> is replaced
document_legacy_probe.js: P5-cov-browser-env (C3e) document.domain fullwidth-digit IPv4 inputs (non-ASCII IP canonicalization)
document_legacy_chrome156.json: P5-cov-browser-env (C3e) recaptured with the fullwidth-digit IPv4 domain inputs
css_selector_syntax.rs: P5-cov-browser-env (C3e) selector lists drop their unreachable non-comma and empty-unforgiving branches
dom_ownership.rs: P5-cov-browser-env (C3e) test-only counted watchdog in retain_helper (cfg(test))
cssom_parser.rs: P5-cov-browser-env (C3e) the declaration parsers share important() for the !important suffix and the end of the value
trusted_types.rs: P5-cov-browser-env (C3e) the sink mapping's OnceLock is a module-level static
cssom_parser.rs: P5-cov-browser-env (C3e) turbo-dom#164 keyframe selectors outside 0%..100% drop their rule
document_legacy.rs: P5-cov-browser-env (C3e) set_domain's IP test reads as 'parses to a non-domain host'
browser_env.rs: P5-cov-browser-env (D1) turbo-dom#114 arg_handle removed (its only caller, contains(), checks its Node argument itself); required-argument counts, contains() / removeAttributeNode() parameter types, setAttribute / toggleAttribute name validation, document / doctype textContent is null
browser_env.js: P5-cov-browser-env (D1) turbo-dom#114 Element / CharacterData ChildNode copies brand-check their interface; scrollIntoView / scrollTo / scrollBy return a ScrollResult promise and convert their dictionaries
document_basics.js: P5-cov-browser-env (D1) turbo-dom#114 DocumentType ChildNode copies brand-check a doctype
css_policy.rs: P5-cov-browser-env (D1) turbo-dom#163 a <style> in a document without a browsing context is allowed a sheet (no policy, nothing cached)
interface_members.js: P5-cov-browser-env (D1) turbo-dom#165 HTMLMediaElement constants are created configurable so key_order.js can place the reflected attributes first
template_contents.rs: P5-cov-browser-env (D2) turbo-dom#151 serialize leaves a pending termination pending (returns nothing)
browser_env.rs: P5-cov-browser-env (D2) turbo-dom#151 dispatch_host_event delivers nothing while a termination is pending
custom_elements.rs: P5-cov-browser-env (D2) turbo-dom#140 no per-call cap on custom element reactions or queued mutation events (Chrome 156 has none; the 4096 caps terminated real pages)
animations.rs: P5-cov-browser-env (D2) turbo-dom#155 keyframe values: %g number text from the token source, var() / env() values keep their specified text, length-only properties Lightning CSS lacks, bare-zero spelling per property grammar, no negative line-height
user_activation_gates.rs: P5-cov-browser-env (D2) turbo-dom#131 created_details: parser-created / cloned open details run the attribute changed steps from closed
custom_elements.rs: P5-cov-browser-env (D2) turbo-dom#131 parser_inserted reports the parsed subtrees' open details
browser_env.rs: P5-cov-browser-env (D2) turbo-dom#131 cloneNode reports the clone's open details
html_popover.rs: P5-cov-browser-env (D2) turbo-dom#139 Chrome 156 exception messages; show / toggle during a show (or a nested hide) throw; a nested hide runs without events; busy is tracked by show / hide themselves
browser_env.js: P5-cov-browser-env (D2) turbo-dom#170 an unchecked longhand the native parser rejects is accepted when its rtdom spelling (math simplified) parses
event_handlers.rs: P5-cov-browser-env (D2) turbo-dom#153 a failed handler compile is kept and reported when the handler is read or its event dispatched
browser_env.rs: P5-cov-browser-env (D2) turbo-dom#153 dispatch reports the path's pending handler compile errors
browser_env.rs: P5-cov-browser-env (D2) turbo-dom#119 the event record keeps the pre-retargeting hitTarget
user_activation_gates.rs: P5-cov-browser-env (D2) turbo-dom#119 light dismiss reads the record's hitTarget
selection.rs: P5-cov-browser-env (D2) turbo-dom#144 text model: default-slot text children, tabs before any rendered table cell, a row / table end's newline only before more content, length-changing and capitalize text-transform in toString
html_invokers.js: D5 turbo-dom#172 activation helpers call captured accessors through a captured Reflect.apply
browser_env.js: D5 turbo-dom#172 the insertion / event / handler paths and turbo-dom#175 the CSSOM use intrinsics captured at install
document_basics.js: D5 turbo-dom#172 check / collection / document element helpers use intrinsics captured at install
selection.js: D5 turbo-dom#144 both table edges keep their caret edge flag
browser_env.rs: D5 turbo-dom#173 Blink's call-without-new messages; turbo-dom#172 captured_intrinsics.js + InlineHandlerSetter
event_handlers.rs: D5 turbo-dom#172 inline handlers set on* through the setter captured at install
html_popover.rs: D5 turbo-dom#171 an inert document schedules on its realm's scheduler
element_relations.rs: A1 safety audit (turbo-dom#183) invariant / page-reachable notes above unwraps
form_pseudo.rs: A1 safety audit (turbo-dom#183) invariant / page-reachable notes above unwraps
key_order.rs: A1 safety audit (turbo-dom#183) invariant / page-reachable notes above unwraps
select_element.rs: A1 safety audit (turbo-dom#183) invariant / page-reachable notes above unwraps
storage_ownership.rs: A1 safety audit (turbo-dom#183) invariant / page-reachable notes above unwraps
typed_om.rs: A1 safety audit (turbo-dom#183) invariant / page-reachable notes above unwraps
shadow_html.js: turbo-dom#199 engine member paths use captured intrinsics
view_transitions.js: turbo-dom#199 engine member paths use captured intrinsics
element_members.js: turbo-dom#199/#222 engine member paths use captured intrinsics and the private engine namespace
form_collections.js: turbo-dom#199/#222 engine member paths use captured intrinsics and the private engine namespace
form_pseudo.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
hyperlink_utils.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
mutation_observers.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
sanitizer.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
script_text.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
shadow_slots.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
template_contents.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
trusted_types.js: turbo-dom#222 engine code reads the private engine namespace (no page-reachable Symbol.for key)
html_popover.js: benv-js r1 popover members call captured intrinsics through Reflect.apply (turbo-dom#194)
element_relations.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
element_tokens.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
interest_invokers.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
focus_group.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
form_outputs.js: benv r4 meter / progress number parsing slices through a captured String.prototype.slice (turbo-dom#199)
reference_target.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
browser_env.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
form_controls.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
typed_om.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
file_inputs.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
input_members.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
html_invokers.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
user_activation_gates.js: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
focus_group_tests.rs: benv-js r2 member paths call captured intrinsics, not page-replaceable builtins (turbo-dom#194)
browser_env.js: D15 captureNodeIntrinsics re-captures members defined after bootstrap (Document#documentElement) and fails install on any non-function entry (turbo-dom#199 regression)
browser_env.rs: D15 namespace_lookup_on_non_element_nodes_matches_chrome156 control test
browser_env.rs: turbo-test#204 wrappers carry a tree generation next to their DOM id (owner_tag/tagged_owner); handle_of refuses a wrapper of a discarded tree or a handle its tree never minted
document_basics.rs: turbo-test#204 adoption tags moved wrappers with the target tree's generation
dom_ownership.rs: turbo-test#204 ownership grouping skips wrappers of a discarded tree
browser_env.rs: turbo-dom#186 XML documents (xml_documents module, parse_xml_document_units, XMLSerializer install; innerHTML / outerHTML of XML documents serialize and parse XML); CDATA sections are named #cdata-section, cloned / imported and read as character data
document_basics.rs: turbo-dom#186 createCDATASection natives (isXML, createCDATA); adoption copies CDATA sections
document_basics.js: turbo-dom#186 createCDATASection creates CDATA sections in XML documents (Chrome's NotSupportedError / InvalidCharacterError messages otherwise)
template_contents.rs: turbo-dom#186 HTML serialization writes a CDATA section as escaped text
animations.js: 0.7.0 turbo-dom#187 CSSAnimation / CSSTransition for declarative animations and transitions, created on style resolution; composite order transitions, CSS animations, script; transition-property all, none under display:none ancestors; 0.7.0 turbo-dom#233 getKeyframes() property order = Blink's hash-map bucket order over Chrome 156 CSSPropertyIDs (scripts/blink-property-ids.py), boundary keyframes per GetComputedKeyframes
browser_env.js: 0.7.0 turbo-dom#187 private ns.cssKeyframes export of @keyframes rule records for CSS animations
animations_tests.rs: 0.7.0 turbo-dom#187 css_animations Chrome 156 control test; control() accepts the frozen headless shell's HeadlessChrome product name; turbo-dom#233 the keyframe order matches Chrome exactly (KNOWN_GAP removed)
browser_env.rs: 0.7.0 turbo-dom#135 installed (page loads and source-position installs included) and parsed documents parse HTML processing instructions like Chrome 156 (Tree::set_processing_instructions / parse_utf16_with_processing_instructions)
sanitizer.rs: 0.7.0 turbo-dom#135 parseHTML documents parse processing instructions like Chrome; Document-level processing instructions are kept, like comments
sanitizer.js: 0.7.0 turbo-dom#135 a safe setHTML dictionary without processing-instruction lists allows none (Chrome 156)
edge_probes_tests.rs: 0.7.0 turbo-dom#135 html_processing_instructions and page_processing_instructions Chrome 156 control tests; the sanitizer probe's processing-instruction KNOWN_GAP rows are gone
browser_env.rs: 0.7.0 turbo-dom#249 the install splits into bootstrap_engine + attach_document (TURBO_DOM_ATTACH_PATH), install scripts are counted for the test gate, and turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
realm_ownership.rs: 0.7.0 turbo-dom#249 DocumentGraph::retire_nodes drops a replaced tree's node records when a document is attached
custom_elements.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
document_legacy.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
event_listeners.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
html_popover.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
mutation_observers.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
range_contents.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
script_state.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
select_element.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
selection.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
trusted_types.rs: 0.7.0 turbo-dom#249 native callbacks are registered as V8 external references through ext! (startup snapshot)
browser_env.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
custom_elements.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
document_legacy.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
editing.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
element_internals.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
event_listeners.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
file_inputs.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
focus_group.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
form_pseudo.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
html_invokers.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
interest_invokers.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
pointer_events.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
range_contents.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
select_element.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
selection.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
user_activation_gates.rs: 0.7.0 turbo-dom#249 function-holding context slots register with the rehydrate registry (fn_slots!) and are captured through context_rehydrate::capture/store (startup snapshot)
browser_env.rs: 0.7.0 turbo-dom#249 snapshot rehydrate: native install halves split out for startup_snapshot::rehydrate, template data keyed through template_data while a snapshot is built
collection_records.rs: 0.7.0 turbo-dom#249 snapshot rehydrate: native install halves split out for startup_snapshot::rehydrate, template data keyed through template_data while a snapshot is built
document_legacy.rs: 0.7.0 turbo-dom#249 snapshot rehydrate: native install halves split out for startup_snapshot::rehydrate, template data keyed through template_data while a snapshot is built
html_popover.rs: 0.7.0 turbo-dom#249 snapshot rehydrate: native install halves split out for startup_snapshot::rehydrate, template data keyed through template_data while a snapshot is built
realm_ownership.rs: 0.7.0 turbo-dom#249 snapshot rehydrate: native install halves split out for startup_snapshot::rehydrate, template data keyed through template_data while a snapshot is built
template_contents.rs: 0.7.0 turbo-dom#249 snapshot rehydrate: native install halves split out for startup_snapshot::rehydrate, template data keyed through template_data while a snapshot is built
dom_isolation.rs: 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
frame_tokens_tests.rs: 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
html_meta_tests.rs: 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
iframe_credentialless_tests.rs: 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
mutation_observers_tests.rs: 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
shadow_slots_tests.rs: 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
range_live.rs: 0.7.0 turbo-dom#249 step 5: the live-range registry is persisted into a snapshot and re-registered on rehydrate
browser_env.rs: 0.7.0 turbo-dom#249 step 5: snapshot_blob / install_from_snapshot / TURBO_DOM_SNAPSHOT_PATH install path; 0.7.0 turbo-dom#249 step 5: test isolates come from tests::new_isolate (the startup snapshot under TURBO_DOM_SNAPSHOT_PATH) or tests::plain_isolate for bootstrap-configuring tests
custom_elements.rs: 0.7.0 turbo-dom#249 step 5: constructor delegates are template_data keys while a snapshot is built
mutation_observers.rs: 0.7.0 turbo-dom#249 step 5: the delivery function's pending array is template data via template_data
html_popover.rs: 0.7.0 turbo-dom#249 step 5: the flush function's data is template data via template_data
collection_records.rs: 0.7.0 turbo-dom#249 step 5: a snapshot isolate adopts the snapshot's record key in whichever context first needs it
reference_target.rs: 0.7.0 turbo-dom#249 step 5: ensure_key for snapshot rehydrate
dom_ownership.rs: 0.7.0 turbo-dom#249 step 5: GC_EPOCH visible to the sweep test
browser_env.js: turbo-dom#254 CSSGroupingRule / CSSConditionRule and the @container / @media / @supports / @layer / @scope / @starting-style rule interfaces (attributes, insertRule / deleteRule, normalized container query)
css_admission_tests.rs: turbo-dom#254 grouping / condition rules vs their Chrome 156 control
browser_env.rs: 0.7.0 R1 #13 run_js compiles through run_engine_script_cached (only fixed engine scripts reach the bytecode cache); R1 #14 the selector memo skips selectors over 512 units and is cleared by each install and reset
browser_env.js: 0.7.0 R1 #15 the node-intrinsics probe div is released after use, so no closure carries it into a startup snapshot
realm_ownership.rs: 0.7.0 R1 #15 retire_nodes leaves only the document in its own ownership component
browser_env.js: 0.7.0 turbo-dom#273 getComputedStyle applies style rules nested in @supports / @layer / @media / @scope (condition evaluation, cascade-layer order), never @container / @starting-style
css_admission_tests.rs: 0.7.0 turbo-dom#273 grouping-rule cascade vs its Chrome 156 control
browser_env.js: 0.7.0 turbo-dom#275 the computed-style cascade orders rules by Selectors 4 specificity; turbo-dom#276 @scope proximity, :scope bound to the scoping root, & as :where(:scope)
browser_env.rs: 0.7.0 turbo-dom#275 __parseCssNative 'specificity' mode (the cascade parser's complex selectors with their specificity)
browser_env.js: 0.7.1 turbo-dom#279 subtree-scoped style invalidation: reused computed styles key on Tree::style_generation by the sheets' selector scope, the style filter, rule-set invalidation of added sheets, animation targets' generations
browser_env.rs: 0.7.1 turbo-dom#279 __computedStyleGeneration / __computedStyleFilter / __computedStyleTouch natives
animations.js: 0.7.1 turbo-dom#279 the effect-target index reports the targets it left and joined to invalidateAnimatedStyles
css_live_tests.rs: 0.7.1 turbo-dom#279 style_invalidation_tests submodule; the reuse test mutates an attribute no style filter skips
browser_env.js: 0.7.1 turbo-dom#307 rule buckets: a cascade skips style rules whose rightmost compound names no class or id the element carries; test-only __noteRuleProbe hook
browser_env.rs: 0.7.1 turbo-dom#307 test-only __noteRuleProbe native (RULE_PROBES counter)
css_live_tests.rs: 0.7.1 turbo-dom#307 __cssTestRuleProbes getter and the rule_bucket_tests submodule
browser_env.js: 0.7.1 turbo-dom#307 part 2 style sharing between same-signature siblings, kept per-sheet rule entries, an unchanged sheet list keeps its identity, the last bucket index extended; turbo-test#227 part 2 cascaded declarations reused under a volatile (animated) style; test-only __noteStyleShare / __styleShareOff hooks
browser_env.rs: 0.7.1 turbo-dom#307 part 2 CSSOM structure epoch natives (__cssomStructureEpoch / __bumpCssomStructure); test-only STYLE_SHARES counter and STYLE_SHARE_OFF switch
css_live_tests.rs: 0.7.1 turbo-dom#307 part 2 __cssTestStyleShares / __cssTestStyleShareOff and the style_sharing_tests submodule
form_checked_state.rs: 0.7.1 perf the checked-radio owner scan runs once per (document, tree version); test-only OWNER_SCANS counter
browser_env.js: 0.7.1 perf the internal-call dispatcher passes arguments as array literals; stable computed styles reused at an unchanged world stamp; the owner's sheet list holds while either tree version does; test-only __noteSheetRead hook
browser_env.rs: 0.7.1 perf test-only STYLE_GENERATION_READS / SHEET_READS counters; the snapshot fingerprint folds in the engine scripts' hash (fingerprint_of)
css_admission_tests.rs: 0.7.0 turbo-dom#275/#276 specificity and @scope proximity vs their Chrome 156 control
control_labels.rs: 0.7.0 perf turbo-dom#271 label lookups read a per-tree-version index (first element per id, label candidates per host chain) + a test-only walk counter
browser_env.js: 0.8.0 turbo-dom#341 the document.createEvent stub moves to html_reflections.js (Chrome's createEvent table)
browser_env.js: 0.8.0 turbo-dom#322 ns.cssSupports / cssSupportsProperty (CSS.supports from the engine grammar), if() is a substitution function, a supports declaration followed by ';' is false
browser_env.rs: 0.8.0 turbo-dom#323 installs html_reflections.js and html_media (html_media.rs/.js) after interface_members.js; interface_parity and css_supports control test modules
element_tokens.js: 0.8.0 turbo-dom#323 ns.defineTokenList and per-list supported tokens (relList / sizes / blocking / controlsList)
element_relations.rs: 0.8.0 turbo-dom#323 a src attribute change runs the media element load algorithm (html_media::attribute_changed)
form_collections.js: 0.8.0 turbo-dom#323 fieldset.elements and datalist.options live collections
cssom_parser.rs: 0.8.0 turbo-dom#322 if() as an arbitrary substitution function with its condition grammar
typed_om.js: 0.8.0 turbo-dom#322 anchor-name / anchor-scope / timeline-scope / position-anchor dashed-ident grammars (fixedGrammar)
user_activation_gates.rs: 0.8.0 turbo-dom#323 ActivationQuery::Sticky and sticky_activation() for media play()'s autoplay policy
computed_defaults_tests.rs: 0.8.0 turbo-dom#323 the test activation host answers ActivationQuery::Sticky
browser_env.rs: 0.8.0 turbo-dom#339 wrapping an audio / video / source / track element turns on html_media's mutation recording (html_media::observe); method and setter calls run html_media's pending insertion steps after the mutation flush; clear_interface_records clears them; html_media_events control test module
mutation_observers.rs: 0.8.0 turbo-dom#339 recording() includes html_media::recording(); childList records reach html_media::mutation
event_listeners.js: 0.8.0 turbo-dom#339 ns.makeEventTarget gives an engine object (html_media.js's TextTrackList) a listener record
browser_env.js: 0.8.0 turbo-dom#318 buttons and button-like inputs compute html.css's ButtonFace background-color, rgb(239, 239, 239)
css_live_tests.rs: 0.8.0 turbo-dom#318 button UA background-color vs its Chrome 156 control
animations_chrome156.json: 0.8.0 turbo-dom#340 recapture with Animation rangeStart / rangeEnd (#369)
trusted_types_mapping.json: 0.8.0 onchecked / onopenerchange map to TrustedScript like Chrome 156 (turbo-surf#154)
trusted_types_chrome155.json: 0.8.0 recaptured in Chrome 156.0.8078.47 with the same pinned cases / probe / collector (the moved path is kept); only onchecked / onopenerchange join the mapping (turbo-surf#154)
document_xpath.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
document_basics.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
mutation_observers.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
html_invokers.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
event_listeners.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
interest_invokers.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
form_outputs.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
element_relations.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
element_internals.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
dom_implementation.js: 0.8.0 turbo-dom#297 cc <= 15 helper extraction (behaviour-preserving)
editing.js: 0.8.0 turbo-dom#389 review: skipEmojiExtendersBackward consumes the emoji base again (Backspace over 👍🏽 / ❤️ / 1️⃣ deletes the sequence, Chrome 156)
editing_tests.rs: 0.8.0 turbo-dom#389 review: the trusted-input control accepts the headless shell's HeadlessChrome/156.0.8078.47
trusted_types.rs: 0.8.0 turbo-dom#389 review: Object.prototype-named namespaces / properties are unknown names (Chrome 156) test
trusted_types.js: 0.8.0 turbo-dom#389 review: KNOWN_NS / SCRIPT_TEXT_PROPS have a null prototype (Object.prototype names are unknown, Chrome 156)
browser_env.rs: 0.8.0 turbo-dom#389 review: initMouseEvent receiver-before-arguments test
browser_env.js: 0.8.0 turbo-dom#389 review: initMouseEvent checks its receiver before the argument count (main / Chrome 156)
browser_env.js: 0.8.0 turbo-dom#389 review: the binding's own text-control selection model is reachable on the private namespace (fallbackControlSelection) for its unit test
browser_env.rs: 0.8.0 turbo-dom#389 review: fallback setRangeText 'preserve' unit test
browser_env.js: 0.8.0 turbo-dom#389 review: setRangeText 'preserve' captures the selection before the value write (main)
browser_env.js: 0.8.0 turbo-dom#389 review: the primordial dispatcher's string / function extras return a shared NOT_HANDLED sentinel instead of a {hit, value} object per call
browser_env.rs: 0.8.0 turbo-dom#389 review: mark_worker_context(scope, context) returns Result and refuses a context that already holds a window (snapshot_blob() contexts included); WindowAlreadyInstalled
editing_input_steps.json: 0.8.0 turbo-dom#389 review: Backspace / execCommand('delete') after x + 👍🏽 / ❤️ / 1️⃣ steps
editing_input_chrome156.json: 0.8.0 turbo-dom#389 review: recaptured with the frozen headless shell (HeadlessChrome/156.0.8078.47, the collector's current flags) over the extended steps; the existing rows are unchanged
browser_env.js: 0.8.0 turbo-dom#389 review 2: validForTypePatterns has a null prototype (<button type=constructor value=x>.value is "x", main / Chrome 156)
browser_env.js: 0.8.0 turbo-dom#389 review 2: the binding's own text-control selection model and its fallbackControlSelection hook are removed (selection.js overwrites every member in every install)
browser_env.rs: 0.8.0 turbo-dom#389 review 2: the fallback setRangeText 'preserve' unit test is removed with the fallback
editing.js: 0.8.0 turbo-dom#389 review 2: skipZwjSequencesBackward takes a skin-tone modifier / FE0F before a ZWJ with its base (Backspace after x + 👨🏽‍💻 leaves x, Chrome 156)
editing_input_steps.json: 0.8.0 turbo-dom#389 review 2: Backspace / execCommand('delete') after x + 👨🏽‍💻 and Backspace after x + ❤️‍🔥 steps
editing_input_chrome156.json: 0.8.0 turbo-dom#389 review 2: recaptured with the frozen headless shell over the extended steps; the 38 existing rows and collectorSha256 are unchanged
event_listeners.rs: 0.8.1 turbo-dom#398 PR-C1 workspace cc <= 15 gate: aborted / error branches of the records native lifted into helpers (behaviour-preserving)
"
# Crate-owned files under src/ that were not part of the move.
OWNED="lib.rs crate_surface.rs host_internals.rs test_support.rs css_snapshot.rs option_factory_probe.js option_factory_cases.json option_factory_chrome156.json html_image.js html_image_tests.rs html_image_probe.js html_image_cases.json html_image_chrome156.json custom_elements_defined_probe.js custom_elements_defined_cases.json custom_elements_defined_chrome156.json computed_defaults_probe.js computed_defaults_cases.json computed_defaults_chrome156.json computed_defaults_tests.rs dom_reactions_names_probe.js dom_reactions_names_cases.json dom_reactions_names_chrome156.json dom_reactions_tests.rs dom_adoption_reactions_probe.js dom_adoption_reactions_cases.json dom_adoption_reactions_chrome156.json shadow_events_probe.js shadow_events_cases.json shadow_events_chrome156.json shadow_events_tests.rs html_track_probe.js html_track_cases.json html_track_chrome156.json html_track_tests.rs html_track.js"
OWNED="$OWNED patched_builtins_members_probe.js patched_builtins_members_chrome156.json patched_builtins_sweep_probe.js patched_builtins_sweep_chrome156.json patched_builtins_sweep_known.txt"
OWNED="$OWNED canvas_style_probe.js canvas_style_chrome156.json canvas_style_tests.rs canvas_state_probe.js canvas_state_chrome156.json canvas_measure_text_probe.js canvas_measure_text_chrome156.json canvas_measure_text_tests.rs canvas_text.rs canvas_text_metrics_interface_probe.js canvas_text_metrics_interface_chrome156.json canvas_text_attributes_probe.js canvas_text_attributes_chrome156.json canvas_spacing_units_probe.js canvas_spacing_units_chrome156.json canvas_ic_unit_probe.js canvas_ic_unit_chrome156.json"
OWNED="$OWNED css_shorthand.rs css_shorthand_tests.rs css_longhand_tests.rs svg_mathml_element_probe.js svg_mathml_element_cases.json svg_mathml_element_chrome156.json svg_mathml_element_tests.rs"
OWNED="$OWNED dom_exception_realm_probe.js dom_exception_realm_cases.json dom_exception_realm_chrome156.json dom_exception_realm_tests.rs custom_elements_collect_tests.rs html_image_srcset_probe.js html_image_srcset_cases.json html_image_srcset_chrome156.json html_image_srcset_tests.rs reflected_urls_probe.js reflected_urls_cases.json reflected_urls_chrome156.json reflected_urls_tests.rs"
OWNED="$OWNED document_domain_probe.js document_domain_cases.json document_domain_chrome156.json"
OWNED="$OWNED dom_ownership_alloc_tests.rs dom_ownership_scale_tests.rs perf_gates_tests.rs"
OWNED="$OWNED install_split_tests.rs external_refs.rs startup_snapshot.rs startup_snapshot_tests.rs context_rehydrate.rs context_rehydrate_tests.rs snapshot_rehydrate_tests.rs template_data.rs"
OWNED="$OWNED engine_namespace_tests.rs engine_namespace_probe.js window_interface_probe.js window_interface_chrome156.json window_interface.js"
OWNED="$OWNED native_guards_probe.js native_guards_chrome156.json native_guards_tests.rs close_watchers_probe.js close_watchers_chrome156.json close_watchers_tests.rs host_api_tests.rs stale_nodes_tests.rs"
OWNED="$OWNED edge_probes_tests.rs harness_tests.rs termination_tests.rs shadow_slots_edges_probe.js shadow_slots_edges_chrome156.json dom_mutation_validity_probe.js dom_mutation_validity_chrome156.json selector_edges_probe.js selector_edges_chrome156.json sanitizer_edges_probe.js sanitizer_edges_chrome156.json document_edges_probe.js document_edges_chrome156.json receivers_edges_probe.js receivers_edges_chrome156.json event_receivers_edges_probe.js event_receivers_edges_chrome156.json"
OWNED="$OWNED activation_granted_probe.js activation_granted_chrome156.json trusted_controls_tests.rs dialog_popover_edges_probe.js dialog_popover_edges_chrome156.json custom_elements_edges_probe.js custom_elements_edges_chrome156.json selection_text_edges_probe.js selection_text_edges_chrome156.json"
OWNED="$OWNED private_natives_tests.rs binding_termination_tests.rs"
OWNED="$OWNED member_edges_tests.rs host_edges_tests.rs member_edges_probe.js member_edges_chrome156.json"
OWNED="$OWNED guard_edges_tests.rs guard_edges_probe.js guard_edges_chrome156.json"
OWNED="$OWNED stack_edge_tests.rs stack_edge_probe.js stack_edge_chrome156.json"
OWNED="$OWNED reflection_regressions_tests.rs"
OWNED="$OWNED native_guards_hostile_probe.js native_guards_hostile_chrome156.json"
OWNED="$OWNED reaction_volume_probe.js reaction_volume_chrome156.json"
OWNED="$OWNED computed_form_background_probe.js computed_form_background_chrome156.json"
OWNED="$OWNED activation_focus_probe.js activation_focus_chrome156.json selector_errors_probe.js selector_errors_chrome156.json"
OWNED="$OWNED keyframe_values_probe.js keyframe_values_chrome156.json"
OWNED="$OWNED style_invalidation_probe.js style_invalidation_chrome156.json style_invalidation_tests.rs"
OWNED="$OWNED rule_bucket_tests.rs style_sharing_tests.rs"
OWNED="$OWNED jsdom_media_jsdom27.json jsdom_media_probe.js media_override_chrome156.json media_override_probe.js"
OWNED="$OWNED jsdom_transitions_chrome156.json jsdom_transitions_jsdom27.json jsdom_transitions_probe.js"
OWNED="$OWNED css_animation_events_probe.js css_animation_events_cases.json css_animation_events_chrome156.json"
OWNED="$OWNED details_toggle_probe.js details_toggle_chrome156.json"
OWNED="$OWNED popover_exceptions_probe.js popover_exceptions_chrome156.json"
OWNED="$OWNED xpath_contexts_probe.js xpath_contexts_chrome156.json xpath_contexts_tests.rs"
OWNED="$OWNED css_animations_cases.json css_animations_chrome156.json css_animations_probe.js processing_instructions_chrome156.json processing_instructions_probe.js page_processing_instructions_chrome156.json"
OWNED="$OWNED page_patched_intrinsics_probe.js captured_intrinsics.js"
OWNED="$OWNED xml_documents.rs xml_documents.js xml_documents_tests.rs xml_documents_probe.js xml_documents_chrome156.json xml_inner_html_probe.js xml_inner_html_chrome156.json"
OWNED="$OWNED event_defaultprevented_chrome156.json"
OWNED="$OWNED css_webkit_prefix_chrome156.json css_webkit_prefix_tests.rs dom_ownership_sweep_tests.rs iframe_inline_handler_realm_chrome156.json"
OWNED="$OWNED css_grouping_rules_probe.js css_grouping_rules_chrome156.json"
OWNED="$OWNED css_grouping_cascade_probe.js css_grouping_cascade_chrome156.json"
OWNED="$OWNED css_cascade_specificity_probe.js css_cascade_specificity_chrome156.json"
OWNED="$OWNED inline_style_reuse_probe.js inline_style_reuse_chrome156.json"
OWNED="$OWNED viewport_cssom_tests.rs custom_property_colors_probe.js custom_property_colors_chrome156.json computed_custom_properties_probe.js computed_custom_properties_chrome156.json match_media_probe.js match_media_chrome156.json iframe_viewport_probe.js iframe_viewport_chrome156.json iframe_media_probe.js iframe_media_chrome156.json iframe_cascade_probe.js iframe_cascade_chrome156.json"
OWNED="$OWNED html_media_events_probe.js html_media_events_cases.json html_media_events_chrome156.json html_media_events_tests.rs"
OWNED="$OWNED html_reflections.js html_media.js html_media.rs interface_parity_probe.js interface_parity_cases.json interface_parity_chrome156.json interface_parity_tests.rs create_event_probe.js create_event_cases.json create_event_chrome156.json css_supports_probe.js css_supports_cases.json css_supports_chrome156.json css_supports_tests.rs"
OWNED="$OWNED listener_errors_probe.js listener_errors_chrome156.json"
OWNED="$OWNED worker_context_tests.rs"
# turbo-dom#338 (tree-counting math, shape()) and #340 (Animation rangeStart / rangeEnd) Chrome 156 controls.
OWNED="$OWNED css_tree_counting_shape_probe.js css_tree_counting_shape_cases.json css_tree_counting_shape_chrome156.json css_tree_counting_shape_tests.rs animation_range_probe.js animation_range_cases.json animation_range_chrome156.json animation_range_tests.rs"
# turbo-dom#358 CSS Nesting Chrome 156 control.
OWNED="$OWNED css_nesting_probe.js css_nesting_cases.json css_nesting_chrome156.json css_nesting_tests.rs"
# turbo-dom#381 / #382 non-Node EventTarget dispatch + microtask checkpoint.
OWNED="$OWNED event_target_dispatch_chrome156.json event_target_dispatch_tests.rs"
# turbo-dom#385 DedicatedWorkerGlobalScope surface + embedder hooks.
OWNED="$OWNED worker_scope.rs worker_scope.js dedicated_worker_global_scope_probe.js dedicated_worker_global_scope_chrome156.json"
# turbo-dom#394 embedder init scripts run before parser inline <script>.
OWNED="$OWNED init_script_tests.rs"

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
