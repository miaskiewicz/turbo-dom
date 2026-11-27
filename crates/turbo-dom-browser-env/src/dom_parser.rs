//! DOMParser uses the same inert native documents as the embedder parser APIs.
use super::*;

fn parse<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue<'s>,
) {
    let Some(document) = sanitizer::realm_document(scope) else {
        return;
    };
    let Some(source) = trusted_types::units(
        scope,
        document,
        args.get(0),
        "TrustedHTML",
        "DOMParser parseFromString",
        "Failed to execute 'parseFromString' on 'DOMParser': ",
        false,
    ) else {
        return;
    };
    let content_type = args.get(1).to_rust_string_lossy(scope);
    let parsed = if content_type == "text/html" {
        parse_html_document_units(scope, &source, false)
    } else {
        parse_xml_document_units(scope, &source, &content_type).document
    };
    if dom_implementation::initialize(scope, parsed, &content_type).is_some() {
        result.set(parsed.into());
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__domParserNative", parse);
    run_js(scope, include_str!("dom_parser.js"));
}
