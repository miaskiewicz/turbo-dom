//! turbo-dom#249 Phase A: the install splits into the tree-independent engine bootstrap and
//! the per-document attach, and attaching a document runs no engine script.
use super::*;

fn scripts_run() -> usize {
    ENGINE_SCRIPTS_RUN.with(std::cell::Cell::get)
}

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn attach_document_runs_no_engine_script() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let before = scripts_run();
    bootstrap_engine(scope);
    let bootstrap = scripts_run() - before;
    assert!(
        bootstrap > 40,
        "the bootstrap runs the engine scripts ({bootstrap})"
    );
    attach_document(
        scope,
        Tree::parse_with_processing_instructions(
            "<!doctype html><title>t</title><body><div id=root onclick='1'>x</div><template><p>c</p></template>",
        ),
    );
    assert_eq!(
        scripts_run() - before,
        bootstrap,
        "attach ran an engine script"
    );
    assert_eq!(
        eval(
            scope,
            "[document.title, document.getElementById('root').textContent, typeof document.body.firstChild.onclick, document.querySelector('template').content.firstChild.localName, document.body.childNodes.length].join()"
        ),
        "t,x,function,p,2"
    );
    reset();
}

#[test]
fn a_full_install_runs_the_bootstrap_once() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let before = scripts_run();
    bootstrap_engine(scope);
    let bootstrap = scripts_run() - before;
    reset();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let before = scripts_run();
    install_with_html(scope, "<!doctype html><body>");
    assert_eq!(scripts_run() - before, bootstrap);
    reset();
}

/// The split path behind `install_with_html`: one bootstrap, then the attach, and the page
/// sees its own document (not the shell) with the parsed state the full install gives it.
#[test]
fn the_split_install_path_attaches_the_parsed_document() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let page = "<!doctype html><body><p id=a>1</p><input type=checkbox checked><div id=h><template shadowrootmode=open><b>s</b></template></div><script nonce=n>1</script>";
    let probe = "[document.getElementById('a').textContent, document.querySelector('input').checked, document.getElementById('h').shadowRoot.innerHTML, document.scripts[0].nonce, document.scripts[0].getAttribute('nonce'), document.body.children.length, document.compatMode].join()";
    let expected = {
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, page);
        let expected = eval(scope, probe);
        reset();
        expected
    };
    assert_eq!(expected, "1,true,<b>s</b>,n,n,4,CSS1Compat");
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    ATTACH_PATH_FOR_TEST.with(|on| on.set(true));
    let before = scripts_run();
    install_with_html(scope, page);
    ATTACH_PATH_FOR_TEST.with(|on| on.set(false));
    let installed = scripts_run() - before;
    assert_eq!(eval(scope, probe), expected);
    reset();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let before = scripts_run();
    bootstrap_engine(scope);
    assert_eq!(
        installed,
        scripts_run() - before,
        "the split path ran more than one bootstrap"
    );
    reset();
}
