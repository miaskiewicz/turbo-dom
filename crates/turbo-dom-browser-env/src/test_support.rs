//! Probe, control and case data of the binding's Chrome-parity tests, for
//! embedders' integration tests (feature `test-support`).
//!
//! Each probe (`*_probe.js`) runs in a page; each control (`*_chrome15x.json`) is
//! the result Chrome recorded for it; `*_cases.json` drive table tests. The files
//! ship in this crate's package (under `src/`); these constants are the library
//! way to reach them, so consumers never copy the files. Names are the file name
//! upper-cased with `.` and `-` replaced by `_`; [`FILES`] maps every file name to
//! its bytes.
//!
//! The set is what turbo-surf's integration tests use today; add a constant here
//! (next to its file) when another consumer needs one.
//!
//! [`frame_read_should_throw`] is the binding's frame-read source of an
//! interceptor's should-throw mode, for embedders' accessor-vs-frame-read
//! agreement tests.

/// The interceptor's should-throw mode read from the V8 frame (the
/// `native-sdk-frame-read` source). Only valid inside a setter, definer or
/// deleter interceptor callback.
pub fn frame_read_should_throw(
    scope: &mut v8::PinScope<'_, '_>,
    args: &v8::PropertyCallbackArguments<'_>,
) -> bool {
    crate::browser_env::interceptor_mode::frame_read_should_throw(scope, args)
}

/// `src/abstract_ranges_chrome155.json`.
pub const ABSTRACT_RANGES_CHROME155_JSON: &str = include_str!("abstract_ranges_chrome155.json");
/// `src/abstract_ranges_frames_chrome155.json`.
pub const ABSTRACT_RANGES_FRAMES_CHROME155_JSON: &str =
    include_str!("abstract_ranges_frames_chrome155.json");
/// `src/abstract_ranges_frames_probe.js`.
pub const ABSTRACT_RANGES_FRAMES_PROBE_JS: &str = include_str!("abstract_ranges_frames_probe.js");
/// `src/abstract_ranges_probe.js`.
pub const ABSTRACT_RANGES_PROBE_JS: &str = include_str!("abstract_ranges_probe.js");
/// `src/animations_frames_cases.json`.
pub const ANIMATIONS_FRAMES_CASES_JSON: &[u8] = include_bytes!("animations_frames_cases.json");
/// `src/animations_frames_chrome156.json`.
pub const ANIMATIONS_FRAMES_CHROME156_JSON: &str = include_str!("animations_frames_chrome156.json");
/// `src/animations_frames_probe.js`.
pub const ANIMATIONS_FRAMES_PROBE_JS: &str = include_str!("animations_frames_probe.js");
/// `src/computed_defaults_cases.json` (turbo-surf#36: UA display / position / visibility
/// defaults, and documents without a parsed `<title>`).
pub const COMPUTED_DEFAULTS_CASES_JSON: &[u8] = include_bytes!("computed_defaults_cases.json");
/// `src/computed_defaults_chrome156.json`.
pub const COMPUTED_DEFAULTS_CHROME156_JSON: &str = include_str!("computed_defaults_chrome156.json");
/// `src/computed_defaults_probe.js` (getComputedStyle defaults + `<head>` contents; sync).
pub const COMPUTED_DEFAULTS_PROBE_JS: &str = include_str!("computed_defaults_probe.js");
/// `src/control_labels_chrome155.json`.
pub const CONTROL_LABELS_CHROME155_JSON: &str = include_str!("control_labels_chrome155.json");
/// `src/control_labels_gc_chrome155.json`.
pub const CONTROL_LABELS_GC_CHROME155_JSON: &str = include_str!("control_labels_gc_chrome155.json");
/// `src/control_labels_gc_probe.js`.
pub const CONTROL_LABELS_GC_PROBE_JS: &str = include_str!("control_labels_gc_probe.js");
/// `src/control_labels_probe.js`.
pub const CONTROL_LABELS_PROBE_JS: &str = include_str!("control_labels_probe.js");
/// `src/control_labels_realm_chrome155.json`.
pub const CONTROL_LABELS_REALM_CHROME155_JSON: &str =
    include_str!("control_labels_realm_chrome155.json");
/// `src/control_labels_realm_probe.js`.
pub const CONTROL_LABELS_REALM_PROBE_JS: &str = include_str!("control_labels_realm_probe.js");
/// `src/css_adoption_chrome155.json`.
pub const CSS_ADOPTION_CHROME155_JSON: &str = include_str!("css_adoption_chrome155.json");
/// `src/css_adoption_lifecycle_chrome155.json`.
pub const CSS_ADOPTION_LIFECYCLE_CHROME155_JSON: &str =
    include_str!("css_adoption_lifecycle_chrome155.json");
/// `src/css_adoption_lifecycle_probe.js`.
pub const CSS_ADOPTION_LIFECYCLE_PROBE_JS: &str = include_str!("css_adoption_lifecycle_probe.js");
/// `src/css_adoption_ownership_chrome155.json`.
pub const CSS_ADOPTION_OWNERSHIP_CHROME155_JSON: &str =
    include_str!("css_adoption_ownership_chrome155.json");
/// `src/css_adoption_ownership_probe.js`.
pub const CSS_ADOPTION_OWNERSHIP_PROBE_JS: &str = include_str!("css_adoption_ownership_probe.js");
/// `src/css_adoption_probe.js`.
pub const CSS_ADOPTION_PROBE_JS: &str = include_str!("css_adoption_probe.js");
/// `src/custom_elements_callbacks_chrome155.json`.
pub const CUSTOM_ELEMENTS_CALLBACKS_CHROME155_JSON: &str =
    include_str!("custom_elements_callbacks_chrome155.json");
/// `src/custom_elements_callbacks_probe.js`.
pub const CUSTOM_ELEMENTS_CALLBACKS_PROBE_JS: &str =
    include_str!("custom_elements_callbacks_probe.js");
/// `src/custom_elements_chrome155.json`.
pub const CUSTOM_ELEMENTS_CHROME155_JSON: &str = include_str!("custom_elements_chrome155.json");
/// `src/custom_elements_defined_cases.json`.
pub const CUSTOM_ELEMENTS_DEFINED_CASES_JSON: &[u8] =
    include_bytes!("custom_elements_defined_cases.json");
/// `src/custom_elements_defined_chrome156.json`.
pub const CUSTOM_ELEMENTS_DEFINED_CHROME156_JSON: &str =
    include_str!("custom_elements_defined_chrome156.json");
/// `src/custom_elements_defined_probe.js`.
pub const CUSTOM_ELEMENTS_DEFINED_PROBE_JS: &str = include_str!("custom_elements_defined_probe.js");
/// `src/custom_elements_extended_chrome155.json`.
pub const CUSTOM_ELEMENTS_EXTENDED_CHROME155_JSON: &str =
    include_str!("custom_elements_extended_chrome155.json");
/// `src/custom_elements_extended_probe.js`.
pub const CUSTOM_ELEMENTS_EXTENDED_PROBE_JS: &str =
    include_str!("custom_elements_extended_probe.js");
/// `src/custom_elements_idl_chrome155.json`.
pub const CUSTOM_ELEMENTS_IDL_CHROME155_JSON: &str =
    include_str!("custom_elements_idl_chrome155.json");
/// `src/custom_elements_idl_probe.js`.
pub const CUSTOM_ELEMENTS_IDL_PROBE_JS: &str = include_str!("custom_elements_idl_probe.js");
/// `src/custom_elements_moves_chrome155.json`.
pub const CUSTOM_ELEMENTS_MOVES_CHROME155_JSON: &str =
    include_str!("custom_elements_moves_chrome155.json");
/// `src/custom_elements_moves_probe.js`.
pub const CUSTOM_ELEMENTS_MOVES_PROBE_JS: &str = include_str!("custom_elements_moves_probe.js");
/// `src/custom_elements_probe.js`.
pub const CUSTOM_ELEMENTS_PROBE_JS: &str = include_str!("custom_elements_probe.js");
/// `src/custom_elements_reactions_chrome155.json`.
pub const CUSTOM_ELEMENTS_REACTIONS_CHROME155_JSON: &str =
    include_str!("custom_elements_reactions_chrome155.json");
/// `src/custom_elements_reactions_probe.js`.
pub const CUSTOM_ELEMENTS_REACTIONS_PROBE_JS: &str =
    include_str!("custom_elements_reactions_probe.js");
/// `src/custom_elements_realms_chrome155.json`.
pub const CUSTOM_ELEMENTS_REALMS_CHROME155_JSON: &str =
    include_str!("custom_elements_realms_chrome155.json");
/// `src/custom_elements_realms_probe.js`.
pub const CUSTOM_ELEMENTS_REALMS_PROBE_JS: &str = include_str!("custom_elements_realms_probe.js");
/// `src/custom_elements_retired_chrome155.json`.
pub const CUSTOM_ELEMENTS_RETIRED_CHROME155_JSON: &str =
    include_str!("custom_elements_retired_chrome155.json");
/// `src/custom_elements_retired_probe.js`.
pub const CUSTOM_ELEMENTS_RETIRED_PROBE_JS: &str = include_str!("custom_elements_retired_probe.js");
/// `src/dom_adoption_reactions_cases.json` (turbo-dom#29 / #42-#44: namespaces, cross-document
/// adoption, template contents, custom element reaction order; shared with the npm runtime's
/// `test/dom-adoption-reactions-chrome.test.mjs`).
pub const DOM_ADOPTION_REACTIONS_CASES_JSON: &str =
    include_str!("dom_adoption_reactions_cases.json");
/// `src/dom_adoption_reactions_chrome156.json`.
pub const DOM_ADOPTION_REACTIONS_CHROME156_JSON: &str =
    include_str!("dom_adoption_reactions_chrome156.json");
/// `src/dom_adoption_reactions_probe.js` (sync; one `new DOMParser().parseFromString(…)`).
pub const DOM_ADOPTION_REACTIONS_PROBE_JS: &str = include_str!("dom_adoption_reactions_probe.js");
/// `src/dom_exception_realm_cases.json` (turbo-surf#42: the page the realm-DOMException probe runs on).
pub const DOM_EXCEPTION_REALM_CASES_JSON: &[u8] = include_bytes!("dom_exception_realm_cases.json");
/// `src/dom_exception_realm_chrome156.json`.
pub const DOM_EXCEPTION_REALM_CHROME156_JSON: &str =
    include_str!("dom_exception_realm_chrome156.json");
/// `src/dom_exception_realm_probe.js` (binding-raised DOMExceptions are the realm's interface; async).
pub const DOM_EXCEPTION_REALM_PROBE_JS: &str = include_str!("dom_exception_realm_probe.js");
/// `src/dom_implementation_chrome155.json`.
pub const DOM_IMPLEMENTATION_CHROME155_JSON: &str =
    include_str!("dom_implementation_chrome155.json");
/// `src/dom_implementation_probe.js`.
pub const DOM_IMPLEMENTATION_PROBE_JS: &str = include_str!("dom_implementation_probe.js");
/// `src/dom_parser_observer_lifecycle_chrome155.json`.
pub const DOM_PARSER_OBSERVER_LIFECYCLE_CHROME155_JSON: &str =
    include_str!("dom_parser_observer_lifecycle_chrome155.json");
/// `src/dom_parser_observer_lifecycle_probe.js`.
pub const DOM_PARSER_OBSERVER_LIFECYCLE_PROBE_JS: &str =
    include_str!("dom_parser_observer_lifecycle_probe.js");
/// `src/dom_reactions_names_cases.json` (turbo-dom#43 / #44 / #54 / #57: fragment-parser
/// reaction order, qualified names, independent-document removals, createDocument names).
pub const DOM_REACTIONS_NAMES_CASES_JSON: &str = include_str!("dom_reactions_names_cases.json");
/// `src/dom_reactions_names_chrome156.json`.
pub const DOM_REACTIONS_NAMES_CHROME156_JSON: &str =
    include_str!("dom_reactions_names_chrome156.json");
/// `src/dom_reactions_names_probe.js`.
pub const DOM_REACTIONS_NAMES_PROBE_JS: &str = include_str!("dom_reactions_names_probe.js");
/// `src/file_inputs_cases.json`.
pub const FILE_INPUTS_CASES_JSON: &str = include_str!("file_inputs_cases.json");
/// `src/file_inputs_chrome156.json`.
pub const FILE_INPUTS_CHROME156_JSON: &str = include_str!("file_inputs_chrome156.json");
/// `src/file_inputs_probe.js`.
pub const FILE_INPUTS_PROBE_JS: &str = include_str!("file_inputs_probe.js");
/// `src/form_collections_cases.json`.
pub const FORM_COLLECTIONS_CASES_JSON: &str = include_str!("form_collections_cases.json");
/// `src/form_collections_lifecycle_chrome155.json`.
pub const FORM_COLLECTIONS_LIFECYCLE_CHROME155_JSON: &str =
    include_str!("form_collections_lifecycle_chrome155.json");
/// `src/form_collections_lifecycle_probe.js`.
pub const FORM_COLLECTIONS_LIFECYCLE_PROBE_JS: &str =
    include_str!("form_collections_lifecycle_probe.js");
/// `src/form_collections_realm_chrome155.json`.
pub const FORM_COLLECTIONS_REALM_CHROME155_JSON: &str =
    include_str!("form_collections_realm_chrome155.json");
/// `src/form_collections_realm_probe.js`.
pub const FORM_COLLECTIONS_REALM_PROBE_JS: &str = include_str!("form_collections_realm_probe.js");
/// `src/formdata_event_chrome155.json`.
pub const FORMDATA_EVENT_CHROME155_JSON: &str = include_str!("formdata_event_chrome155.json");
/// `src/formdata_event_probe.js`.
pub const FORMDATA_EVENT_PROBE_JS: &str = include_str!("formdata_event_probe.js");
/// `src/formdata_guards_chrome155.json`.
pub const FORMDATA_GUARDS_CHROME155_JSON: &str = include_str!("formdata_guards_chrome155.json");
/// `src/formdata_guards_probe.js`.
pub const FORMDATA_GUARDS_PROBE_JS: &str = include_str!("formdata_guards_probe.js");
/// `src/html_image_cases.json` (turbo-surf#32 / turbo-dom#26: the parsed images, the decodable
/// one's natural size and the rendered images' layout boxes, `layout`: id -> [width, height, x, y]).
pub const HTML_IMAGE_CASES_JSON: &[u8] = include_bytes!("html_image_cases.json");
/// `src/html_image_chrome156.json`.
pub const HTML_IMAGE_CHROME156_JSON: &str = include_str!("html_image_chrome156.json");
/// `src/html_image_probe.js` (HTMLImageElement image-state and width / height / x / y accessors; async).
pub const HTML_IMAGE_PROBE_JS: &str = include_str!("html_image_probe.js");
/// `src/html_image_srcset_cases.json` (turbo-dom#71: the srcset / sizes / `<picture>` page,
/// viewport 1024x768; image URLs `/img/<W>x<H>/<name>.png`).
pub const HTML_IMAGE_SRCSET_CASES_JSON: &[u8] = include_bytes!("html_image_srcset_cases.json");
/// `src/html_image_srcset_chrome156.json` (`load1` / `load2`: DPR 1 / 2, every image loads;
/// `none1`: nothing loads).
pub const HTML_IMAGE_SRCSET_CHROME156_JSON: &str = include_str!("html_image_srcset_chrome156.json");
/// `src/html_image_srcset_probe.js` (run after the load event).
pub const HTML_IMAGE_SRCSET_PROBE_JS: &str = include_str!("html_image_srcset_probe.js");
/// `src/html_track_cases.json` (turbo-dom#74: HTMLTrackElement / TextTrack).
pub const HTML_TRACK_CASES_JSON: &str = include_str!("html_track_cases.json");
/// `src/html_track_chrome156.json`.
pub const HTML_TRACK_CHROME156_JSON: &str = include_str!("html_track_chrome156.json");
/// `src/html_track_probe.js` (sync).
pub const HTML_TRACK_PROBE_JS: &str = include_str!("html_track_probe.js");
/// `src/key_order_chrome156.json`.
pub const KEY_ORDER_CHROME156_JSON: &str = include_str!("key_order_chrome156.json");
/// `src/key_order_probe.js`.
pub const KEY_ORDER_PROBE_JS: &str = include_str!("key_order_probe.js");
/// `src/radio_dirty_adoption_gc_chrome155.json`.
pub const RADIO_DIRTY_ADOPTION_GC_CHROME155_JSON: &str =
    include_str!("radio_dirty_adoption_gc_chrome155.json");
/// `src/radio_dirty_adoption_gc_probe.js`.
pub const RADIO_DIRTY_ADOPTION_GC_PROBE_JS: &str = include_str!("radio_dirty_adoption_gc_probe.js");
/// `src/radio_dirty_cases.json`.
pub const RADIO_DIRTY_CASES_JSON: &str = include_str!("radio_dirty_cases.json");
/// `src/radio_dirty_gc_chrome155.json`.
pub const RADIO_DIRTY_GC_CHROME155_JSON: &str = include_str!("radio_dirty_gc_chrome155.json");
/// `src/radio_dirty_gc_probe.js`.
pub const RADIO_DIRTY_GC_PROBE_JS: &str = include_str!("radio_dirty_gc_probe.js");
/// `src/shadow_events_cases.json` (turbo-dom#66: event paths across shadow trees).
pub const SHADOW_EVENTS_CASES_JSON: &str = include_str!("shadow_events_cases.json");
/// `src/shadow_events_chrome156.json`.
pub const SHADOW_EVENTS_CHROME156_JSON: &str = include_str!("shadow_events_chrome156.json");
/// `src/shadow_events_probe.js` (sync).
pub const SHADOW_EVENTS_PROBE_JS: &str = include_str!("shadow_events_probe.js");
/// `src/template_lifecycle_chrome155.json`.
pub const TEMPLATE_LIFECYCLE_CHROME155_JSON: &str =
    include_str!("template_lifecycle_chrome155.json");
/// `src/template_lifecycle_probe.js`.
pub const TEMPLATE_LIFECYCLE_PROBE_JS: &str = include_str!("template_lifecycle_probe.js");
/// `src/template_ownership_chrome155.json`.
pub const TEMPLATE_OWNERSHIP_CHROME155_JSON: &str =
    include_str!("template_ownership_chrome155.json");
/// `src/template_ownership_probe.js`.
pub const TEMPLATE_OWNERSHIP_PROBE_JS: &str = include_str!("template_ownership_probe.js");
/// `src/view_transitions_chrome156.json`.
pub const VIEW_TRANSITIONS_CHROME156_JSON: &str = include_str!("view_transitions_chrome156.json");
/// `src/view_transitions_hidden_probe.js`.
pub const VIEW_TRANSITIONS_HIDDEN_PROBE_JS: &str = include_str!("view_transitions_hidden_probe.js");
/// `src/view_transitions_probe.js`.
pub const VIEW_TRANSITIONS_PROBE_JS: &str = include_str!("view_transitions_probe.js");
/// `src/xml_factory_elements_chrome155.json`.
pub const XML_FACTORY_ELEMENTS_CHROME155_JSON: &str =
    include_str!("xml_factory_elements_chrome155.json");
/// `src/xml_factory_elements_probe.js`.
pub const XML_FACTORY_ELEMENTS_PROBE_JS: &str = include_str!("xml_factory_elements_probe.js");

/// Every constant above, keyed by its file name under `src/`.
pub const FILES: &[(&str, &[u8])] = &[
    (
        "abstract_ranges_chrome155.json",
        ABSTRACT_RANGES_CHROME155_JSON.as_bytes(),
    ),
    (
        "abstract_ranges_frames_chrome155.json",
        ABSTRACT_RANGES_FRAMES_CHROME155_JSON.as_bytes(),
    ),
    (
        "abstract_ranges_frames_probe.js",
        ABSTRACT_RANGES_FRAMES_PROBE_JS.as_bytes(),
    ),
    (
        "abstract_ranges_probe.js",
        ABSTRACT_RANGES_PROBE_JS.as_bytes(),
    ),
    ("animations_frames_cases.json", ANIMATIONS_FRAMES_CASES_JSON),
    (
        "animations_frames_chrome156.json",
        ANIMATIONS_FRAMES_CHROME156_JSON.as_bytes(),
    ),
    (
        "animations_frames_probe.js",
        ANIMATIONS_FRAMES_PROBE_JS.as_bytes(),
    ),
    ("computed_defaults_cases.json", COMPUTED_DEFAULTS_CASES_JSON),
    (
        "computed_defaults_chrome156.json",
        COMPUTED_DEFAULTS_CHROME156_JSON.as_bytes(),
    ),
    (
        "computed_defaults_probe.js",
        COMPUTED_DEFAULTS_PROBE_JS.as_bytes(),
    ),
    (
        "control_labels_chrome155.json",
        CONTROL_LABELS_CHROME155_JSON.as_bytes(),
    ),
    (
        "control_labels_gc_chrome155.json",
        CONTROL_LABELS_GC_CHROME155_JSON.as_bytes(),
    ),
    (
        "control_labels_gc_probe.js",
        CONTROL_LABELS_GC_PROBE_JS.as_bytes(),
    ),
    (
        "control_labels_probe.js",
        CONTROL_LABELS_PROBE_JS.as_bytes(),
    ),
    (
        "control_labels_realm_chrome155.json",
        CONTROL_LABELS_REALM_CHROME155_JSON.as_bytes(),
    ),
    (
        "control_labels_realm_probe.js",
        CONTROL_LABELS_REALM_PROBE_JS.as_bytes(),
    ),
    (
        "css_adoption_chrome155.json",
        CSS_ADOPTION_CHROME155_JSON.as_bytes(),
    ),
    (
        "css_adoption_lifecycle_chrome155.json",
        CSS_ADOPTION_LIFECYCLE_CHROME155_JSON.as_bytes(),
    ),
    (
        "css_adoption_lifecycle_probe.js",
        CSS_ADOPTION_LIFECYCLE_PROBE_JS.as_bytes(),
    ),
    (
        "css_adoption_ownership_chrome155.json",
        CSS_ADOPTION_OWNERSHIP_CHROME155_JSON.as_bytes(),
    ),
    (
        "css_adoption_ownership_probe.js",
        CSS_ADOPTION_OWNERSHIP_PROBE_JS.as_bytes(),
    ),
    ("css_adoption_probe.js", CSS_ADOPTION_PROBE_JS.as_bytes()),
    (
        "custom_elements_callbacks_chrome155.json",
        CUSTOM_ELEMENTS_CALLBACKS_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_callbacks_probe.js",
        CUSTOM_ELEMENTS_CALLBACKS_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_chrome155.json",
        CUSTOM_ELEMENTS_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_defined_cases.json",
        CUSTOM_ELEMENTS_DEFINED_CASES_JSON,
    ),
    (
        "custom_elements_defined_chrome156.json",
        CUSTOM_ELEMENTS_DEFINED_CHROME156_JSON.as_bytes(),
    ),
    (
        "custom_elements_defined_probe.js",
        CUSTOM_ELEMENTS_DEFINED_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_extended_chrome155.json",
        CUSTOM_ELEMENTS_EXTENDED_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_extended_probe.js",
        CUSTOM_ELEMENTS_EXTENDED_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_idl_chrome155.json",
        CUSTOM_ELEMENTS_IDL_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_idl_probe.js",
        CUSTOM_ELEMENTS_IDL_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_moves_chrome155.json",
        CUSTOM_ELEMENTS_MOVES_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_moves_probe.js",
        CUSTOM_ELEMENTS_MOVES_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_probe.js",
        CUSTOM_ELEMENTS_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_reactions_chrome155.json",
        CUSTOM_ELEMENTS_REACTIONS_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_reactions_probe.js",
        CUSTOM_ELEMENTS_REACTIONS_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_realms_chrome155.json",
        CUSTOM_ELEMENTS_REALMS_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_realms_probe.js",
        CUSTOM_ELEMENTS_REALMS_PROBE_JS.as_bytes(),
    ),
    (
        "custom_elements_retired_chrome155.json",
        CUSTOM_ELEMENTS_RETIRED_CHROME155_JSON.as_bytes(),
    ),
    (
        "custom_elements_retired_probe.js",
        CUSTOM_ELEMENTS_RETIRED_PROBE_JS.as_bytes(),
    ),
    (
        "dom_adoption_reactions_cases.json",
        DOM_ADOPTION_REACTIONS_CASES_JSON.as_bytes(),
    ),
    (
        "dom_adoption_reactions_chrome156.json",
        DOM_ADOPTION_REACTIONS_CHROME156_JSON.as_bytes(),
    ),
    (
        "dom_adoption_reactions_probe.js",
        DOM_ADOPTION_REACTIONS_PROBE_JS.as_bytes(),
    ),
    (
        "dom_exception_realm_cases.json",
        DOM_EXCEPTION_REALM_CASES_JSON,
    ),
    (
        "dom_exception_realm_chrome156.json",
        DOM_EXCEPTION_REALM_CHROME156_JSON.as_bytes(),
    ),
    (
        "dom_exception_realm_probe.js",
        DOM_EXCEPTION_REALM_PROBE_JS.as_bytes(),
    ),
    (
        "dom_implementation_chrome155.json",
        DOM_IMPLEMENTATION_CHROME155_JSON.as_bytes(),
    ),
    (
        "dom_implementation_probe.js",
        DOM_IMPLEMENTATION_PROBE_JS.as_bytes(),
    ),
    (
        "dom_parser_observer_lifecycle_chrome155.json",
        DOM_PARSER_OBSERVER_LIFECYCLE_CHROME155_JSON.as_bytes(),
    ),
    (
        "dom_parser_observer_lifecycle_probe.js",
        DOM_PARSER_OBSERVER_LIFECYCLE_PROBE_JS.as_bytes(),
    ),
    (
        "dom_reactions_names_cases.json",
        DOM_REACTIONS_NAMES_CASES_JSON.as_bytes(),
    ),
    (
        "dom_reactions_names_chrome156.json",
        DOM_REACTIONS_NAMES_CHROME156_JSON.as_bytes(),
    ),
    (
        "dom_reactions_names_probe.js",
        DOM_REACTIONS_NAMES_PROBE_JS.as_bytes(),
    ),
    ("file_inputs_cases.json", FILE_INPUTS_CASES_JSON.as_bytes()),
    (
        "file_inputs_chrome156.json",
        FILE_INPUTS_CHROME156_JSON.as_bytes(),
    ),
    ("file_inputs_probe.js", FILE_INPUTS_PROBE_JS.as_bytes()),
    (
        "form_collections_cases.json",
        FORM_COLLECTIONS_CASES_JSON.as_bytes(),
    ),
    (
        "form_collections_lifecycle_chrome155.json",
        FORM_COLLECTIONS_LIFECYCLE_CHROME155_JSON.as_bytes(),
    ),
    (
        "form_collections_lifecycle_probe.js",
        FORM_COLLECTIONS_LIFECYCLE_PROBE_JS.as_bytes(),
    ),
    (
        "form_collections_realm_chrome155.json",
        FORM_COLLECTIONS_REALM_CHROME155_JSON.as_bytes(),
    ),
    (
        "form_collections_realm_probe.js",
        FORM_COLLECTIONS_REALM_PROBE_JS.as_bytes(),
    ),
    (
        "formdata_event_chrome155.json",
        FORMDATA_EVENT_CHROME155_JSON.as_bytes(),
    ),
    (
        "formdata_event_probe.js",
        FORMDATA_EVENT_PROBE_JS.as_bytes(),
    ),
    (
        "formdata_guards_chrome155.json",
        FORMDATA_GUARDS_CHROME155_JSON.as_bytes(),
    ),
    (
        "formdata_guards_probe.js",
        FORMDATA_GUARDS_PROBE_JS.as_bytes(),
    ),
    ("html_image_cases.json", HTML_IMAGE_CASES_JSON),
    (
        "html_image_chrome156.json",
        HTML_IMAGE_CHROME156_JSON.as_bytes(),
    ),
    ("html_image_probe.js", HTML_IMAGE_PROBE_JS.as_bytes()),
    ("html_image_srcset_cases.json", HTML_IMAGE_SRCSET_CASES_JSON),
    (
        "html_image_srcset_chrome156.json",
        HTML_IMAGE_SRCSET_CHROME156_JSON.as_bytes(),
    ),
    (
        "html_image_srcset_probe.js",
        HTML_IMAGE_SRCSET_PROBE_JS.as_bytes(),
    ),
    ("html_track_cases.json", HTML_TRACK_CASES_JSON.as_bytes()),
    (
        "html_track_chrome156.json",
        HTML_TRACK_CHROME156_JSON.as_bytes(),
    ),
    ("html_track_probe.js", HTML_TRACK_PROBE_JS.as_bytes()),
    (
        "key_order_chrome156.json",
        KEY_ORDER_CHROME156_JSON.as_bytes(),
    ),
    ("key_order_probe.js", KEY_ORDER_PROBE_JS.as_bytes()),
    (
        "radio_dirty_adoption_gc_chrome155.json",
        RADIO_DIRTY_ADOPTION_GC_CHROME155_JSON.as_bytes(),
    ),
    (
        "radio_dirty_adoption_gc_probe.js",
        RADIO_DIRTY_ADOPTION_GC_PROBE_JS.as_bytes(),
    ),
    ("radio_dirty_cases.json", RADIO_DIRTY_CASES_JSON.as_bytes()),
    (
        "radio_dirty_gc_chrome155.json",
        RADIO_DIRTY_GC_CHROME155_JSON.as_bytes(),
    ),
    (
        "radio_dirty_gc_probe.js",
        RADIO_DIRTY_GC_PROBE_JS.as_bytes(),
    ),
    (
        "shadow_events_cases.json",
        SHADOW_EVENTS_CASES_JSON.as_bytes(),
    ),
    (
        "shadow_events_chrome156.json",
        SHADOW_EVENTS_CHROME156_JSON.as_bytes(),
    ),
    ("shadow_events_probe.js", SHADOW_EVENTS_PROBE_JS.as_bytes()),
    (
        "template_lifecycle_chrome155.json",
        TEMPLATE_LIFECYCLE_CHROME155_JSON.as_bytes(),
    ),
    (
        "template_lifecycle_probe.js",
        TEMPLATE_LIFECYCLE_PROBE_JS.as_bytes(),
    ),
    (
        "template_ownership_chrome155.json",
        TEMPLATE_OWNERSHIP_CHROME155_JSON.as_bytes(),
    ),
    (
        "template_ownership_probe.js",
        TEMPLATE_OWNERSHIP_PROBE_JS.as_bytes(),
    ),
    (
        "view_transitions_chrome156.json",
        VIEW_TRANSITIONS_CHROME156_JSON.as_bytes(),
    ),
    (
        "view_transitions_hidden_probe.js",
        VIEW_TRANSITIONS_HIDDEN_PROBE_JS.as_bytes(),
    ),
    (
        "view_transitions_probe.js",
        VIEW_TRANSITIONS_PROBE_JS.as_bytes(),
    ),
    (
        "xml_factory_elements_chrome155.json",
        XML_FACTORY_ELEMENTS_CHROME155_JSON.as_bytes(),
    ),
    (
        "xml_factory_elements_probe.js",
        XML_FACTORY_ELEMENTS_PROBE_JS.as_bytes(),
    ),
];
