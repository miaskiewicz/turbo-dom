//! The `browser_env` items embedders call today (turbo-test's runner:
//! install/enabled/reset; turbo-surf: everything else) stay public. Compile-time
//! check: a renamed or demoted item fails this test's build.
#![allow(unused_imports)]

use turbo_dom_browser_env::browser_env::{
    activate_isolate, begin_form_data_constructor_entries, capture_file_host,
    capture_popover_scheduler, capture_view_transition_host, capture_window_event_interfaces,
    clear_context_capabilities, clear_interface_records, collect_native_storage,
    composed_contains_node, create_document_element, defer_key_order, dispatch_formdata_event,
    dispatch_host_event, document_html, enabled, engine_namespace, event_callback_state,
    finish_form_data_constructor, finish_key_order, form_submission_base_href,
    form_submission_base_target, get_interface_record, hide_node_nonce, host_pointer,
    import_document_node, install, install_css_policy, install_form_submission_handler,
    install_form_value_reader, install_html, install_independent_html,
    install_independent_html_units, install_independent_srcdoc_units,
    install_independent_utf16_source_positions, install_inline_event_handlers,
    install_mutation_consumer, install_trusted_types_policy, install_with_html,
    install_with_html_source_positions, is_light_document_node, is_native_abort_object,
    is_native_event, is_native_event_target, is_native_xpath_object, light_dismiss,
    mark_event_trusted, modal_backdrop, native_event_target_has_listener, node_attribute,
    node_has_window_owner, node_nonce, node_parser_position, notify_user_activation,
    parse_html_document_units, parser_scripts, prepare_script, process_close_watchers,
    register_dom_exception, register_native_event_target, remove_mutation_consumer, reset,
    reset_isolate, retire_custom_element_context, run_engine_script, script_document,
    set_activation_host, set_document_domain_sandboxed, set_document_last_modified,
    set_document_origin_keyed, set_document_resource_active, set_form_submission_blocked,
    set_inert_document_initializer, set_interface_record, set_native_event_target_observer,
    string_code_units, sweep_collected_documents, trusted_script_source, trusted_type_string,
    ActivationQuery, NativeDomScope, PointerLockPolicy, ENGINE_NAMESPACE_PARAM,
};

#[test]
fn embedder_surface_is_public() {
    assert!(enabled());
}

fn init_v8() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn eval_string(scope: &mut v8::PinScope, source: &str) -> String {
    let code = v8::String::new(scope, source).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    value.to_rust_string_lossy(scope)
}

#[test]
fn install_html_document_html_and_srcdoc_documents() {
    init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    assert_eq!(document_html(), "");
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_html(scope, "<p id=a>x</p>");
    eval_string(
        scope,
        "document.getElementById('a').textContent = 'changed'",
    );
    let html = document_html();
    assert!(html.starts_with("<html><head>"), "{html}");
    assert!(
        html.ends_with("<body><p id=\"a\">changed</p></body></html>"),
        "{html}"
    );
    // Without a doctype an ordinary document is in quirks mode; a srcdoc
    // document never is.
    let quirks = v8::Context::new(scope, Default::default());
    {
        let scope = &mut v8::ContextScope::new(scope, quirks);
        let units: Vec<u16> = "<p>q</p>".encode_utf16().collect();
        install_independent_html_units(scope, &units);
        assert_eq!(eval_string(scope, "document.compatMode"), "BackCompat");
    }
    for source_positions in [false, true] {
        let srcdoc = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, srcdoc);
        let units: Vec<u16> = "<p>s</p>".encode_utf16().collect();
        install_independent_srcdoc_units(scope, &units, source_positions);
        assert_eq!(eval_string(scope, "document.compatMode"), "CSS1Compat");
        assert_eq!(eval_string(scope, "document.body.textContent"), "s");
    }
    reset();
}
