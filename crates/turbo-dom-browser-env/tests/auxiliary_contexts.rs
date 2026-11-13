#![cfg(feature = "host-internals")]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;

use turbo_dom_browser_env::{browser_env, host_internals::*};

#[derive(Default)]
struct HostState {
    windows: RefCell<Vec<v8::Global<v8::Context>>>,
    requests: RefCell<Vec<OpenRequest>>,
    closes: RefCell<usize>,
    deny: RefCell<bool>,
    ready_mode: Cell<u8>,
    fail_surface: Cell<bool>,
    events: RefCell<Vec<(&'static str, AuxiliaryContextId)>>,
}

struct TargetId(AuxiliaryContextId);

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
        v8::V8::initialize();
    });
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, code: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, code).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn admit<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    selected: Option<v8::Local<'s, v8::Context>>,
    request: &OpenRequest,
) -> Option<v8::Local<'s, v8::Context>> {
    let host = scope.get_slot::<Rc<HostState>>().cloned().unwrap();
    host.requests.borrow_mut().push(request.clone());
    if *host.deny.borrow() {
        return None;
    }
    if let Some(selected) = selected {
        return Some(selected);
    }
    let context = create_auxiliary_window_context(scope, None);
    context.set_slot(Rc::new(TargetId(request.target_id)));
    let origin = source.get_security_token(scope);
    context.set_security_token(origin);
    {
        let scope = &mut v8::ContextScope::new(scope, context);
        initialize_auxiliary_document(scope, "<!doctype html><body><p id=child>child</p>");
        if host.fail_surface.get() {
            let name = v8::String::new(scope, "name").unwrap();
            let value = v8::String::new(scope, "locked").unwrap();
            context.global(scope).define_own_property(
                scope,
                name.into(),
                value.into(),
                v8::PropertyAttribute::DONT_DELETE,
            );
        }
    }
    host.windows
        .borrow_mut()
        .push(v8::Global::new(scope, context));
    Some(context)
}

fn close(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    let host = scope.get_slot::<Rc<HostState>>().unwrap();
    *host.closes.borrow_mut() += 1;
    if let Some(id) = context.get_slot::<TargetId>() {
        host.events.borrow_mut().push(("close", id.0));
    }
    true
}

fn ready<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _: v8::Local<'s, v8::Context>,
    target: v8::Local<'s, v8::Context>,
    request: &OpenRequest,
) -> AuxiliaryReadyAction {
    let host = scope.get_slot::<Rc<HostState>>().cloned().unwrap();
    if auxiliary_context_identity(target).map(|id| id.id) != Some(request.target_id)
        || scope.get_current_context() != target
    {
        return AuxiliaryReadyAction::Abort;
    }
    host.events.borrow_mut().push(("ready", request.target_id));
    let source = match host.ready_mode.get() {
        1 => return AuxiliaryReadyAction::Abort,
        2 => "throw new Error('ready boom')",
        3 => "globalThis.readyFlag=true;close();",
        _ => "globalThis.readyFlag=true;if(opener&&!opener.opener)globalThis.nested=open();",
    };
    let Some(code) = v8::String::new(scope, source) else {
        return AuxiliaryReadyAction::Abort;
    };
    if v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .is_none()
    {
        return AuxiliaryReadyAction::Abort;
    }
    AuxiliaryReadyAction::InitializedByHost
}

fn assert_js(scope: &mut v8::PinScope, code: &str) {
    assert!(eval(scope, code).is_true(), "{code}");
}

#[test]
fn native_auxiliary_identity_admission_and_close_are_consumer_usable() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let context = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, context);
        let origin = v8::String::new(scope, "https://example.test").unwrap();
        context.set_security_token(origin.into());
        browser_env::install_independent_html(scope, "<!doctype html><body>opener</body>");
        assert!(install_auxiliary_context(
            scope,
            context,
            "https://example.test/path/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        let root_id = auxiliary_context_identity(context).unwrap().id;
        assert_js(scope, "['closed','opener','name','length'].every(key=>{const d=Object.getOwnPropertyDescriptor(window,key);return d.get.name==='get '+key&&d.get.length===0&&(!d.set||(d.set.name==='set '+key&&d.set.length===1))})");
        assert_js(scope, "globalThis.popup=open('report','Report');popup!==window&&popup.window===popup&&popup.self===popup&&popup.parent===popup&&popup.top===popup&&popup.opener===window&&popup.document.getElementById('child').textContent==='child'&&!popup.closed");
        assert_eq!(
            host.requests.borrow()[0].url,
            "https://example.test/path/report"
        );
        assert_js(
            scope,
            "open('', 'Report')===popup&&open('', 'report')!==popup",
        );
        assert_eq!(host.windows.borrow().len(), 2);
        assert_js(
            scope,
            "popup.name='renamed';open('', 'renamed')===popup&&open('', 'Report')!==popup",
        );
        assert_js(
            scope,
            "open('', 'isolated','noopener')===null&&open('', 'isolated','noreferrer')===null",
        );
        let isolated = v8::Local::new(scope, &host.windows.borrow()[3]);
        assert!(auxiliary_context_opener(scope, isolated).is_none());
        let isolated_identity = auxiliary_context_identity(isolated).unwrap();
        assert_eq!(isolated_identity.creation_source, Some(root_id));
        assert!(isolated_identity.auxiliary && !isolated_identity.closed);
        assert!(host.requests.borrow().last().unwrap().noreferrer);
        assert_js(scope, "popup.opener=null;popup.opener===null");
        assert_js(
            scope,
            "popup.close();popup.close();popup.closed&&window.closed===false",
        );
        assert_eq!(*host.closes.borrow(), 1);
        assert_js(scope, "open('', 'renamed')!==popup");
        *host.deny.borrow_mut() = true;
        let count = host.windows.borrow().len();
        assert_js(scope, "open('/denied')===null");
        assert_eq!(host.windows.borrow().len(), count);
        assert_js(scope, "document.body.textContent==='opener'");
        let calls = host.requests.borrow().len();
        assert_js(scope, "(()=>{try{open({toString(){throw Error('convert')} })}catch(e){return e.message==='convert'}return false})()");
        assert_eq!(host.requests.borrow().len(), calls);
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn staged_initialization_defers_guest_code_until_canonical_identity_exists() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        browser_env::add_init_script("globalThis.initCount=(globalThis.initCount||0)+1;globalThis.initHasOpen=typeof open==='function';globalThis.initHasOpener=!!opener;");
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert_js(scope, "typeof initCount==='undefined'");
        assert!(!run_auxiliary_init_scripts(scope, root));
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        {
            let document = v8::Local::<v8::Object>::try_from(eval(scope, "document")).unwrap();
            let owner = document_id_of(scope, document);
            let _native = enter_context_document(root).unwrap();
            assert_eq!(current_document_id(), owner);
            let native_handle = handle_of(scope, document).unwrap();
            assert_eq!(with_tree(|tree| tree.root()), Some(native_handle));
            assert_eq!(
                with_tree(|tree| tree.text_content(tree.query_selector("body").unwrap())).unwrap(),
                "root"
            );
        }
        assert!(run_auxiliary_init_scripts(scope, root));
        assert!(!run_auxiliary_init_scripts(scope, root));
        assert_js(
            scope,
            "initCount===1&&initHasOpen===true&&initHasOpener===false",
        );
        assert_js(
            scope,
            "globalThis.popup=open();popup.initCount===1&&popup.opener===window",
        );
        let child = v8::Local::new(scope, &host.windows.borrow()[0]);
        assert!(!run_auxiliary_init_scripts(scope, child));
        assert!(!run_auxiliary_init_scripts(scope, child));
        assert_js(
            scope,
            "popup.initCount===1&&popup.initHasOpen===true&&popup.initHasOpener===true",
        );
        browser_env::clear_init_scripts();
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn isolated_commit_keeps_page_identity_but_closes_the_openers_old_proxy() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    isolate.set_slot(Rc::new(HostState::default()));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://opener.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://opener.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        assert_js(
            scope,
            "globalThis.oldPopup=open('', 'report');oldPopup.opener=null;!oldPopup.closed",
        );
        let host = scope.get_slot::<Rc<HostState>>().cloned().unwrap();
        let previous = v8::Local::new(scope, &host.windows.borrow()[0]);
        let identity = auxiliary_context_identity(previous).unwrap();
        assert_eq!(
            identity.creation_source,
            Some(auxiliary_context_identity(root).unwrap().id)
        );
        let current = create_auxiliary_window_context(scope, None);
        {
            let scope = &mut v8::ContextScope::new(scope, current);
            let token = v8::String::new(scope, "https://isolated.test").unwrap();
            current.set_security_token(token.into());
            initialize_auxiliary_document(scope, "<!doctype html><body>isolated</body>");
        }
        assert!(commit_isolated_auxiliary_navigation(
            scope,
            previous,
            current,
            "https://isolated.test/"
        ));
        assert_eq!(auxiliary_context_closed(previous), Some(true));
        let active = auxiliary_context_identity(current).unwrap();
        assert_eq!(active.id, identity.id);
        assert_eq!(active.creation_source, identity.creation_source);
        assert!(!active.closed);
        assert_js(scope, "oldPopup.closed&&open('', 'report')!==oldPopup");
        {
            let scope = &mut v8::ContextScope::new(scope, current);
            assert_js(
                scope,
                "!closed&&opener===null&&name===''&&document.body.textContent==='isolated'",
            );
        }
        assert_eq!(*host.closes.borrow(), 0);
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn closing_opener_keeps_auxiliary_documents_and_actor_usable() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        assert_js(scope, "open()!==null");
        let child = v8::Local::new(scope, &host.windows.borrow()[0]);
        assert!(retire_auxiliary_context(scope, root));
        assert!(!retire_auxiliary_context(scope, root));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            assert_js(scope, "!closed&&opener.closed&&document.getElementById('child').textContent==='child'&&open()!==null");
        }
        assert_eq!(host.windows.borrow().len(), 2);
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn source_positioned_staged_documents_preserve_native_csp_provenance() {
    init();
    for primary in [true, false] {
        let mut isolate = v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        {
            v8::scope!(let scope, &mut isolate);
            let template = auxiliary_window_template(scope);
            let context = v8::Context::new(
                scope,
                v8::ContextOptions {
                    global_template: Some(template),
                    ..Default::default()
                },
            );
            let scope = &mut v8::ContextScope::new(scope, context);
            let prefix = "😀\r\n<script nonce='real'>";
            let source = format!("{prefix}globalThis.parsedRan=true</script>");
            let units = source.encode_utf16().collect::<Vec<_>>();
            browser_env::add_init_script("globalThis.initRan=(globalThis.initRan||0)+1");
            if primary {
                initialize_root_auxiliary_utf16_source_positions(scope, &units);
            } else {
                initialize_auxiliary_utf16_source_positions(scope, &units);
            }
            assert!(install_auxiliary_context(
                scope,
                context,
                "https://example.test/",
                None
            ));
            assert_js(
                scope,
                "typeof initRan==='undefined'&&typeof parsedRan==='undefined'",
            );
            let document = v8::Local::<v8::Object>::try_from(eval(scope, "document")).unwrap();
            let scripts = browser_env::parser_scripts(scope, document);
            assert_eq!(scripts.len(), 1);
            let script = scripts[0];
            let position = browser_env::node_parser_position(scope, script, None).unwrap();
            assert_eq!(position.utf16_offset, prefix.encode_utf16().count());
            assert_eq!(position.line, 2);
            assert_eq!(
                position.column,
                "<script nonce='real'>".encode_utf16().count() + 1
            );
            assert_eq!(
                browser_env::node_nonce(scope, script).as_deref(),
                Some("real")
            );
            assert_eq!(
                browser_env::node_parser_position(scope, script, Some("nonce")),
                Some(position)
            );
            assert!(run_auxiliary_init_scripts(scope, context));
            assert_js(scope, "initRan===1&&typeof parsedRan==='undefined'");
            assert_eq!(
                browser_env::node_parser_position(scope, script, None),
                Some(position)
            );
            browser_env::clear_init_scripts();
        }
    }
}

#[test]
fn queued_init_for_a_retired_context_cannot_consume_its_replacements_init() {
    init();
    for same_origin in [true, false] {
        let mut isolate = v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        let host = Rc::new(HostState::default());
        isolate.set_slot(Rc::clone(&host));
        {
            v8::scope!(let scope, &mut isolate);
            let template = auxiliary_window_template(scope);
            let root = v8::Context::new(
                scope,
                v8::ContextOptions {
                    global_template: Some(template),
                    ..Default::default()
                },
            );
            let scope = &mut v8::ContextScope::new(scope, root);
            let token = v8::String::new(scope, "https://example.test").unwrap();
            root.set_security_token(token.into());
            initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
            assert!(install_auxiliary_context(
                scope,
                root,
                "https://example.test/",
                Some(AuxiliaryHost { open: admit, close })
            ));
            browser_env::add_init_script("globalThis.initCount=(globalThis.initCount||0)+1");
            assert_js(scope, "open()!==null");
            let previous = v8::Local::new(scope, &host.windows.borrow()[0]);
            let current = create_auxiliary_window_context(scope, Some(previous));
            {
                let scope = &mut v8::ContextScope::new(scope, current);
                let token = v8::String::new(
                    scope,
                    if same_origin {
                        "https://example.test"
                    } else {
                        "https://other.test"
                    },
                )
                .unwrap();
                current.set_security_token(token.into());
                initialize_auxiliary_document(scope, "<!doctype html><body>replacement</body>");
            }
            assert!(commit_auxiliary_navigation(
                scope,
                previous,
                current,
                if same_origin {
                    "https://example.test/new"
                } else {
                    "https://other.test/new"
                }
            ));
            let stale_request = host.requests.borrow()[0].clone();
            let request_count = host.requests.borrow().len();
            assert!(open_auxiliary_context(scope, previous, &stale_request).is_none());
            assert_eq!(host.requests.borrow().len(), request_count);
            assert!(!run_auxiliary_init_scripts(scope, previous));
            assert!(run_auxiliary_init_scripts(scope, current));
            assert!(!run_auxiliary_init_scripts(scope, current));
            {
                let scope = &mut v8::ContextScope::new(scope, current);
                assert_js(
                    scope,
                    "initCount===1&&document.body.textContent==='replacement'",
                );
            }
            browser_env::clear_init_scripts();
            host.windows.borrow_mut().clear();
        }
        isolate.remove_slot::<Rc<HostState>>();
    }
}

#[test]
fn retired_auxiliary_is_collectible_after_host_and_guest_release_it() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        let weak = {
            v8::scope!(let inner, scope);
            assert_js(
                inner,
                "globalThis.held=open('', 'collectible');held.close();held.closed",
            );
            let child = v8::Local::new(inner, &host.windows.borrow()[0]);
            v8::Weak::new(inner, child)
        };
        host.windows.borrow_mut().clear();
        assert_js(scope, "held=null;document.body.textContent==='root'");
        for _ in 0..3 {
            scope.low_memory_notification();
            browser_env::flush_ownership(scope);
        }
        assert!(
            weak.is_empty(),
            "weak name/context registries must not retain a closed popup"
        );
        assert_eq!(auxiliary_context_closed(root), Some(false));
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn host_ready_precedes_opener_access_and_supports_reentrant_context_only_init() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        set_auxiliary_ready_host(root, Some(ready));
        browser_env::add_init_script("globalThis.openerPageOnly=true");
        assert_js(scope, "globalThis.popup=open();popup.readyFlag===true&&popup.nested.readyFlag===true&&popup.nested.opener===popup&&typeof popup.openerPageOnly==='undefined'&&typeof popup.nested.openerPageOnly==='undefined'");
        let requests = host.requests.borrow();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            *host.events.borrow(),
            vec![
                ("ready", requests[0].target_id),
                ("ready", requests[1].target_id)
            ]
        );
        assert_eq!(requests[1].source_id, requests[0].target_id);
        drop(requests);
        let child = v8::Local::new(scope, &host.windows.borrow()[0]);
        assert!(!run_auxiliary_init_scripts(scope, child));
        browser_env::clear_init_scripts();
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn ready_publication_precedes_init_close_and_closes_only_the_owned_child() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    host.ready_mode.set(3);
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        set_auxiliary_ready_host(root, Some(ready));
        assert_js(scope, "globalThis.popup=open();popup!==null&&popup.closed&&popup.readyFlag&&closed===false&&document.body.textContent==='root'");
        let target = host.requests.borrow()[0].target_id;
        assert_eq!(
            *host.events.borrow(),
            vec![("ready", target), ("close", target)]
        );
        assert_eq!(*host.closes.borrow(), 1);
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn failed_ready_never_falls_back_to_opener_scripts_and_releases_the_owned_target() {
    init();
    for mode in [1, 2] {
        let mut isolate = v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        let host = Rc::new(HostState::default());
        host.ready_mode.set(mode);
        isolate.set_slot(Rc::clone(&host));
        {
            v8::scope!(let scope, &mut isolate);
            let template = auxiliary_window_template(scope);
            let root = v8::Context::new(
                scope,
                v8::ContextOptions {
                    global_template: Some(template),
                    ..Default::default()
                },
            );
            let scope = &mut v8::ContextScope::new(scope, root);
            let token = v8::String::new(scope, "https://example.test").unwrap();
            root.set_security_token(token.into());
            initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
            assert!(install_auxiliary_context(
                scope,
                root,
                "https://example.test/",
                Some(AuxiliaryHost { open: admit, close })
            ));
            set_auxiliary_ready_host(root, Some(ready));
            browser_env::add_init_script("globalThis.openerPageOnly=true");
            assert_js(
                scope,
                if mode == 1 {
                    "open()===null"
                } else {
                    "(()=>{try{open();return false}catch(error){return error.message==='ready boom'}})()"
                },
            );
            let child = v8::Local::new(scope, &host.windows.borrow()[0]);
            assert_eq!(auxiliary_context_closed(child), Some(true));
            assert_eq!(*host.closes.borrow(), 1);
            {
                let scope = &mut v8::ContextScope::new(scope, child);
                assert_js(scope, "typeof openerPageOnly==='undefined'");
            }
            assert_eq!(auxiliary_context_closed(root), Some(false));
            browser_env::clear_init_scripts();
            host.windows.borrow_mut().clear();
        }
        isolate.remove_slot::<Rc<HostState>>();
    }
}

#[test]
fn failed_surface_never_publishes_an_identity_or_reuses_its_name() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    host.fail_surface.set(true);
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        set_auxiliary_ready_host(root, Some(ready));
        assert_js(
            scope,
            "(()=>{try{return open('', 'failed')===null}catch(_){return true}})()",
        );
        let failed = v8::Local::new(scope, &host.windows.borrow()[0]);
        assert!(auxiliary_context_identity(failed).is_none());
        assert!(host.events.borrow().iter().all(|event| event.0 != "ready"));
        assert_eq!(*host.closes.borrow(), 1);
        host.fail_surface.set(false);
        assert_js(scope, "open('', 'failed').readyFlag===true");
        assert!(host.requests.borrow()[1].new_context);
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn staged_auxiliary_initializers_preserve_the_worker_window_opt_out() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        browser_env::mark_worker_context(scope, context).unwrap();
        initialize_auxiliary_document(scope, "<body>must not install</body>");
        initialize_root_auxiliary_document(scope, "<body>must not install</body>");
        let units = "<script>must_not_run()</script>"
            .encode_utf16()
            .collect::<Vec<_>>();
        initialize_auxiliary_utf16_source_positions(scope, &units);
        initialize_root_auxiliary_utf16_source_positions(scope, &units);
        assert_js(
            scope,
            "typeof window==='undefined'&&typeof document==='undefined'",
        );
        assert!(!install_auxiliary_context(
            scope,
            context,
            "https://worker.test/",
            None
        ));
    }
}

#[test]
fn named_noopener_and_reserved_targets_match_the_recorded_chrome_rules() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let root = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, root);
        let token = v8::String::new(scope, "https://example.test").unwrap();
        root.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            root,
            "https://example.test/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        assert_js(scope, "['_self','_parent','_top'].every(target=>open('',target)===window&&opener===null&&open('',target,'noopener')===window&&opener===null)");
        assert!(host.windows.borrow().is_empty());
        assert_js(scope, "globalThis.existing=open('', 'Named');existing.opener===window&&open('', 'Named','noopener')===null&&existing.name==='Named'&&existing.opener===window");
        assert_eq!(host.windows.borrow().len(), 1);
        assert!(!host.requests.borrow().last().unwrap().new_context);
        let existing = v8::Local::new(scope, &host.windows.borrow()[0]);
        assert!(auxiliary_context_opener(scope, existing)
            .unwrap()
            .strict_equals(root.global(scope).into()));
        assert_js(scope, "open('', 'Fresh','noopener')===null");
        assert_eq!(host.windows.borrow().len(), 2);
        let fresh = v8::Local::new(scope, &host.windows.borrow()[1]);
        assert!(auxiliary_context_opener(scope, fresh).is_none());
        {
            let scope = &mut v8::ContextScope::new(scope, fresh);
            assert_js(scope, "name==='Fresh'&&opener===null");
        }
        host.windows.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<HostState>>();
}

#[test]
fn native_root_profile_query_rejects_ordinary_context_and_copied_prototype() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    v8::scope!(let scope, &mut isolate);
    let template = auxiliary_window_template(scope);
    let native = v8::Context::new(
        scope,
        v8::ContextOptions {
            global_template: Some(template),
            ..Default::default()
        },
    );
    let ordinary = v8::Context::new(scope, Default::default());
    let token = v8::String::new(scope, "https://example.test").unwrap();
    native.set_security_token(token.into());
    ordinary.set_security_token(token.into());
    let scope = &mut v8::ContextScope::new(scope, native);
    assert!(is_auxiliary_window_context(scope, native));
    assert!(!is_auxiliary_window_context(scope, ordinary));
    let prototype = native.global(scope).get_prototype(scope).unwrap();
    let ordinary_global = ordinary.global(scope);
    assert_eq!(ordinary_global.set_prototype(scope, prototype), Some(true));
    assert!(!is_auxiliary_window_context(scope, ordinary));
    let scope = &mut v8::ContextScope::new(scope, ordinary);
    initialize_root_auxiliary_document(scope, "<body>ordinary page</body>");
    assert!(!install_auxiliary_context(
        scope,
        ordinary,
        "https://example.test/",
        None
    ));
    assert!(auxiliary_context_identity(ordinary).is_none());
    assert_js(scope, "document.body.textContent==='ordinary page'");
}

fn coverage_context<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Context> {
    let context = create_auxiliary_window_context(scope, None);
    let token = v8::String::new(scope, "https://frames.test").unwrap();
    context.set_security_token(token.into());
    let scope = &mut v8::ContextScope::new(scope, context);
    initialize_auxiliary_document(scope, "<!doctype html><body>frame</body>");
    context
}

#[test]
fn nested_sources_keep_parent_top_named_targets_and_fallback_urls_distinct() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    v8::scope!(let scope, &mut isolate);
    let host_context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, host_context);
    let root = coverage_context(scope);
    assert!(install_auxiliary_context(
        scope,
        root,
        "https://frames.test/root/",
        Some(AuxiliaryHost { open: admit, close })
    ));
    let frame = coverage_context(scope);
    assert!(install_nested_auxiliary_source(
        scope,
        frame,
        root,
        "https://frames.test/frame/",
        &"frame".encode_utf16().collect::<Vec<_>>(),
        Some(AuxiliaryHost { open: admit, close })
    ));
    let leaf = coverage_context(scope);
    assert!(install_nested_auxiliary_source(
        scope,
        leaf,
        frame,
        "https://frames.test/leaf/",
        &"leaf".encode_utf16().collect::<Vec<_>>(),
        Some(AuxiliaryHost { open: admit, close })
    ));
    assert!(!auxiliary_context_identity(frame).unwrap().auxiliary);
    assert_eq!(
        auxiliary_document_url(leaf).as_deref(),
        Some("https://frames.test/leaf/")
    );
    assert_eq!(
        auxiliary_document_fallback_base(leaf).as_deref(),
        Some("https://frames.test/leaf/")
    );
    {
        let scope = &mut v8::ContextScope::new(scope, leaf);
        let key = v8::String::new(scope, "rootWindow").unwrap();
        let root_global = root.global(scope);
        leaf.global(scope)
            .set(scope, key.into(), root_global.into());
        let key = v8::String::new(scope, "parentWindow").unwrap();
        let frame_global = frame.global(scope);
        leaf.global(scope)
            .set(scope, key.into(), frame_global.into());
        assert_js(scope, "open('', '_self')===window&&open('', '_parent')===parentWindow&&open('', '_top')===rootWindow&&open('', 'frame')===parentWindow");
        assert_js(scope, "globalThis.child=open('relative','nested-popup');child.opener===window&&child.parent===child&&child.top===child&&length===0");
    }
    let popup = v8::Local::new(scope, host.windows.borrow().last().unwrap());
    assert_eq!(
        auxiliary_document_url(popup).as_deref(),
        Some("about:blank")
    );
    assert_eq!(
        auxiliary_document_fallback_base(popup).as_deref(),
        Some("https://frames.test/leaf/")
    );
    assert_eq!(
        host.requests.borrow().last().unwrap().url,
        "https://frames.test/leaf/relative"
    );
    assert_eq!(
        auxiliary_context_identity(popup).unwrap().creation_source,
        Some(auxiliary_context_identity(leaf).unwrap().id)
    );
    host.windows.borrow_mut().clear();
}

#[test]
fn host_severance_retires_name_selection_without_closing_the_popup() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    v8::scope!(let scope, &mut isolate);
    let host_context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, host_context);
    let root = coverage_context(scope);
    assert!(install_auxiliary_context(
        scope,
        root,
        "https://frames.test/",
        Some(AuxiliaryHost { open: admit, close })
    ));
    let scope = &mut v8::ContextScope::new(scope, root);
    assert_js(
        scope,
        "globalThis.first=open('','severed');first.opener===window",
    );
    let popup = v8::Local::new(scope, &host.windows.borrow()[0]);
    let identity = auxiliary_context_identity(popup).unwrap();
    assert!(sever_auxiliary_opener(scope, popup));
    assert!(auxiliary_context_opener(scope, popup).is_none());
    assert_eq!(auxiliary_context_identity(popup).unwrap().id, identity.id);
    assert_js(
        scope,
        "first.opener===null&&!first.closed&&open('','severed')!==first",
    );
    assert_eq!(*host.closes.borrow(), 0);
    assert_eq!(host.windows.borrow().len(), 2);
    host.windows.borrow_mut().clear();
}

#[test]
fn unregistered_and_duplicate_host_context_operations_fail_without_identity_changes() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    v8::scope!(let scope, &mut isolate);
    let host_context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, host_context);
    let root = coverage_context(scope);
    let other = coverage_context(scope);
    assert!(!install_nested_auxiliary_source(
        scope,
        other,
        root,
        "https://frames.test/",
        &[],
        None
    ));
    assert!(!run_auxiliary_init_scripts(scope, other));
    assert!(!sever_auxiliary_opener(scope, other));
    assert!(!retire_auxiliary_context(scope, other));
    assert!(!commit_auxiliary_navigation(
        scope,
        other,
        root,
        "https://frames.test/"
    ));
    assert!(!commit_isolated_auxiliary_navigation(
        scope,
        other,
        root,
        "https://frames.test/"
    ));
    assert!(auxiliary_document_url(other).is_none());
    assert!(auxiliary_document_fallback_base(other).is_none());
    assert!(install_auxiliary_context(
        scope,
        root,
        "https://frames.test/",
        None
    ));
    let id = auxiliary_context_identity(root).unwrap().id;
    let empty = create_auxiliary_window_context(scope, None);
    assert!(!install_auxiliary_context(
        scope,
        empty,
        "https://frames.test/",
        None
    ));
    assert!(!install_nested_auxiliary_source(
        scope,
        empty,
        root,
        "https://frames.test/",
        &[],
        None
    ));
    assert!(!commit_isolated_auxiliary_navigation(
        scope,
        root,
        empty,
        "https://frames.test/"
    ));
    assert!(!install_auxiliary_context(
        scope,
        root,
        "https://different.test/",
        None
    ));
    assert!(!install_nested_auxiliary_source(
        scope,
        root,
        root,
        "https://frames.test/",
        &[],
        None
    ));
    assert!(!commit_auxiliary_navigation(
        scope,
        root,
        other,
        "https://frames.test/"
    ));
    assert!(!commit_isolated_auxiliary_navigation(
        scope,
        root,
        root,
        "https://frames.test/"
    ));
    let scope = &mut v8::ContextScope::new(scope, root);
    set_auxiliary_ready_host(root, Some(ready));
    set_auxiliary_ready_host(root, None);
    assert_js(scope, "open()===null&&!closed");
    assert_eq!(auxiliary_context_identity(root).unwrap().id, id);
    assert_eq!(
        auxiliary_document_url(root).as_deref(),
        Some("https://frames.test/")
    );
}

#[test]
fn window_open_converts_all_arguments_before_admission_and_rejects_illegal_receivers() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    v8::scope!(let scope, &mut isolate);
    let host_context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, host_context);
    let root = coverage_context(scope);
    assert!(install_auxiliary_context(
        scope,
        root,
        "https://frames.test/",
        Some(AuxiliaryHost { open: admit, close })
    ));
    let scope = &mut v8::ContextScope::new(scope, root);
    assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(window,'name').get.call({})}catch(e){return e.name==='TypeError'}return false})()&&['name','opener'].every(key=>{const d=Object.getOwnPropertyDescriptor(window,key);try{d.set.call({},null);return false}catch(e){return e.name==='TypeError'}})");
    assert_js(
        scope,
        "(()=>{try{open.call({})}catch(e){return e.name==='TypeError'}return false})()",
    );
    assert_js(scope, "[1,2].every(index=>{const args=['','_blank',''];args[index]={toString(){throw Error('argument '+index)}};try{open(...args)}catch(e){return e.message==='argument '+index}return false})");
    assert_js(
        scope,
        "(()=>{try{open('http://[')}catch(e){return e.name==='SyntaxError'}return false})()",
    );
    assert!(host.requests.borrow().is_empty());
    set_auxiliary_ready_host(root, Some(ready));
    set_auxiliary_ready_host(root, None);
    assert_js(scope, "globalThis.child=open();child.opener={hostMarker:7};child.opener.hostMarker===7&&length===0");
    assert_eq!(host.requests.borrow().len(), 1);
    assert!(host.events.borrow().is_empty());
    host.windows.borrow_mut().clear();
}

#[test]
fn hyperlink_helper_uses_native_base_target_and_rejects_nonlinks() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    let host = Rc::new(HostState::default());
    isolate.set_slot(Rc::clone(&host));
    v8::scope!(let scope, &mut isolate);
    let host_context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, host_context);
    let root = coverage_context(scope);
    assert!(install_auxiliary_context(
        scope,
        root,
        "https://frames.test/root/",
        Some(AuxiliaryHost { open: admit, close })
    ));
    let scope = &mut v8::ContextScope::new(scope, root);
    eval(scope, "document.head.innerHTML='<base href=\"/assets/\" target=\"report\">';document.body.innerHTML='<a id=a href=\"page\">link</a><a id=empty>empty</a><div id=div>div</div>'");
    for code in [
        "document.getElementById('empty')",
        "document.getElementById('div')",
        "document.createElement('a')",
        "({})",
    ] {
        let value = eval(scope, code);
        let value = v8::Local::<v8::Object>::try_from(value).unwrap();
        assert!(open_auxiliary_hyperlink(scope, value).is_none());
    }
    assert!(host.requests.borrow().is_empty());
    let anchor = eval(scope, "document.getElementById('a')");
    let anchor = v8::Local::<v8::Object>::try_from(anchor).unwrap();
    assert!(open_auxiliary_hyperlink(scope, anchor).is_some());
    let request = host.requests.borrow().last().unwrap().clone();
    assert_eq!(request.url, "https://frames.test/assets/page");
    assert_eq!(String::from_utf16(&request.target).unwrap(), "report");
    assert!(!request.noopener);
    host.windows.borrow_mut().clear();
}
