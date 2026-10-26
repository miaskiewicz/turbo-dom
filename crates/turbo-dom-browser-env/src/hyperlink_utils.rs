//! HTMLHyperlinkElementUtils (`<a>`/`<area>` origin, protocol, host, …, hash and toString).
//!
//! The element's url is its resolved `href`, parsed with the WHATWG parser; the components and
//! setters are the URL standard's own (`url::quirks`), so `a.hash = …` behaves like `URL#hash`.
//! The JS half owns brand checks and writes the re-serialised URL back to the `href` attribute.
use super::*;

fn component(url: &url::Url, name: &str) -> String {
    use url::quirks;
    match name {
        "origin" => quirks::origin(url),
        "protocol" => quirks::protocol(url).to_owned(),
        "username" => quirks::username(url).to_owned(),
        "password" => quirks::password(url).to_owned(),
        "host" => quirks::host(url).to_owned(),
        "hostname" => quirks::hostname(url).to_owned(),
        "port" => quirks::port(url).to_owned(),
        "pathname" => quirks::pathname(url).to_owned(),
        "search" => quirks::search(url).to_owned(),
        "hash" => quirks::hash(url).to_owned(),
        _ => String::new(),
    }
}

fn update(url: &mut url::Url, name: &str, value: &str) {
    use url::quirks;
    // Each setter is a no-op on input the URL standard rejects, like the URL API.
    let _ = match name {
        "protocol" => quirks::set_protocol(url, value),
        "username" => quirks::set_username(url, value),
        "password" => quirks::set_password(url, value),
        "host" => quirks::set_host(url, value),
        "hostname" => quirks::set_hostname(url, value),
        "port" => quirks::set_port(url, value),
        "pathname" => {
            quirks::set_pathname(url, value);
            Ok(())
        }
        "search" => {
            quirks::set_search(url, value);
            Ok(())
        }
        "hash" => {
            quirks::set_hash(url, value);
            Ok(())
        }
        _ => Ok(()),
    };
}

/// The element's url: its `href` attribute parsed against the document base URL (a `<base
/// href>` resolved against the document URL), or None without an attribute or on failure.
fn element_url(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) -> Option<url::Url> {
    let _dom = DomScope::for_object(scope, element);
    let handle = handle_of(scope, element)?;
    let href = with_tree(|tree| tree.get_attribute(handle, "href").map(str::to_owned)).flatten()?;
    let document_url = event_handlers::document_url();
    let base = form_submission_base_href(scope, element)
        .and_then(|base| url::Url::parse(&document_url).ok()?.join(&base).ok())
        .or_else(|| url::Url::parse(&document_url).ok());
    url::Url::options()
        .base_url(base.as_ref())
        .parse(&href)
        .ok()
}

/// `(operation, element, component, value?)`: "get" returns the component of the element's url
/// (or the null-url value Chrome reports); "set" returns the updated href, or null when there is
/// no url to update. The JS half has already brand-checked the element.
fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = arg_str(scope, &args, 0);
    let Ok(element) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return rv.set_null();
    };
    let parsed = element_url(scope, element);
    let name = arg_str(scope, &args, 2);
    match (operation.as_str(), parsed) {
        ("get", Some(url)) => {
            let value = v8::String::new(scope, &component(&url, &name)).unwrap();
            rv.set(value.into());
        }
        // Chrome reports ":" for the protocol of an element without a valid url.
        ("get", None) => {
            let value = v8::String::new(scope, if name == "protocol" { ":" } else { "" }).unwrap();
            rv.set(value.into());
        }
        ("set", Some(mut url)) => {
            update(&mut url, &name, &arg_str(scope, &args, 3));
            let value = v8::String::new(scope, url.as_str()).unwrap();
            rv.set(value.into());
        }
        _ => rv.set_null(),
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__hyperlinkNative", dispatch);
    run_js(scope, include_str!("hyperlink_utils.js"));
}
