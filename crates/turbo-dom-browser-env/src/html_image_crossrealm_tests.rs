//! Shared per-image records and decode ownership across genuine native realms.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("html_image_adoption_probe.js");
const CONTROL: &str = include_str!("html_image_adoption_chrome156.json");

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn host<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'s, v8::Context>,
) -> v8::Local<'s, v8::Value> {
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let key = v8::String::new(scope, "document").unwrap();
    let document = global
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let record = collection_records::get(scope, document)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "imageHost").unwrap();
    record.get(scope, key.into()).unwrap()
}

fn with_realms(run: impl FnOnce(&mut v8::PinScope)) {
    tests::init_v8();
    let isolate = &mut tests::new_isolate();
    v8::scope!(let scope, isolate);
    let root = v8::Context::new(scope, Default::default());
    let child = v8::Context::new(scope, Default::default());
    let token = v8::String::new(scope, "same-image-origin").unwrap();
    root.set_security_token(token.into());
    child.set_security_token(token.into());
    for context in [root, child] {
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
    }
    let scope = &mut v8::ContextScope::new(scope, root);
    let root_host = host(scope, root);
    let child_host = host(scope, child);
    let global = root.global(scope);
    for (name, value) in [
        ("imagePeerForTest", child.global(scope).into()),
        ("rootImageHostForTest", root_host),
        ("childImageHostForTest", child_host),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        global.set(scope, key.into(), value);
    }
    let image = eval(scope, "document.createElement('img')")
        .to_object(scope)
        .unwrap();
    let record = v8::Object::new(scope);
    let key = v8::String::new(scope, "unrelated").unwrap();
    let value = v8::Integer::new(scope, 17);
    record.create_data_property(scope, key.into(), value.into());
    assert!(collection_records::set(scope, image, record.into()));
    let key = v8::String::new(scope, "seededImageForTest").unwrap();
    global.set(scope, key.into(), image.into());
    run(scope);
    let key = v8::String::new(scope, "unrelated").unwrap();
    assert_eq!(
        record.get(scope, key.into()).unwrap().int32_value(scope),
        Some(17)
    );
    reset();
}

fn settled(scope: &mut v8::PinScope, source: &str) -> serde_json::Value {
    eval(scope, &format!("Promise.resolve({source}).then(value=>globalThis.resultForTest=JSON.stringify(value),error=>globalThis.resultForTest=JSON.stringify({{error:String(error.stack)}}))"));
    scope.perform_microtask_checkpoint();
    let value = eval(scope, "resultForTest").to_rust_string_lossy(scope);
    serde_json::from_str(&value).unwrap()
}

#[test]
fn image_state_follows_adoption_and_decode_method_realms() {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    with_realms(|scope| {
        for host in ["rootImageHostForTest", "childImageHostForTest"] {
            let load=format!("(image,good)=>{{const host={host};host.setState(image,{{complete:false}});return Promise.resolve().then(()=>host.setState(image,good?{{complete:true,naturalWidth:3,naturalHeight:2,currentSrc:'https://image.test/pixel.png'}}:null));}}");
            let probe = PROBE.trim().trim_end_matches(';');
            let actual = settled(scope, &format!("({probe})(imagePeerForTest,{load})"));
            assert_eq!(actual, control["result"], "host={host}");
        }
    });
}

#[test]
fn image_private_records_and_decode_queue_ignore_authored_prototype_setters() {
    with_realms(|scope| {
        let result = settled(
            scope,
            r#"(async()=>{
          const image=seededImageForTest;document.body.append(image);
          let calls=0;const descriptor={configurable:true,set(){calls++;throw Error('prototype setter ran');}};
          Object.defineProperty(Object.prototype,'imageState',descriptor);
          Object.defineProperty(Object.prototype,'imagePendingDecodes',descriptor);
          Object.defineProperty(Array.prototype,'0',descriptor);
          let pending;
          try { rootImageHostForTest.setState(image,{complete:false});pending=HTMLImageElement.prototype.decode.call(image);rootImageHostForTest.setState(image,{complete:true,naturalWidth:3,naturalHeight:2,currentSrc:'pixel'}); }
          finally {delete Object.prototype.imageState;delete Object.prototype.imagePendingDecodes;delete Array.prototype[0];}
          await pending;return {calls,width:image.naturalWidth};
        })()"#,
        );
        assert_eq!(result, serde_json::json!({"calls":0,"width":3}));
    });
}

#[test]
fn shared_image_decode_records_do_not_root_released_native_realms() {
    tests::init_v8();
    let isolate = &mut tests::new_isolate();
    v8::scope!(let scope, isolate);
    let root = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, root);
    install_independent_html(scope, "<!doctype html><body><p>healthy</p>");
    let (context_weak, document_weak, image_weak) = {
        v8::scope!(let inner, scope);
        let child = v8::Context::new(inner, Default::default());
        let inner = &mut v8::ContextScope::new(inner, child);
        install_independent_html(inner, "<!doctype html><body>");
        let image_host = host(inner, child);
        let key = v8::String::new(inner, "imageHostForTest").unwrap();
        child.global(inner).set(inner, key.into(), image_host);
        let image = eval(inner, "(()=>{const image=document.createElement('img');document.body.append(image);imageHostForTest.setState(image,{complete:false});image.decode();image.remove();return image;})()").to_object(inner).unwrap();
        let document = eval(inner, "document").to_object(inner).unwrap();
        (
            v8::Weak::new(inner, child),
            v8::Weak::new(inner, document),
            v8::Weak::new(inner, image),
        )
    };
    flush_ownership(scope);
    for _ in 0..3 {
        scope.low_memory_notification();
    }
    assert!(
        image_weak.is_empty(),
        "private pending-decode record retained image"
    );
    assert!(
        document_weak.is_empty(),
        "private pending-decode record retained document"
    );
    assert!(
        context_weak.is_empty(),
        "private pending-decode record retained realm"
    );
    assert_eq!(
        eval(scope, "document.body.textContent").to_rust_string_lossy(scope),
        "healthy"
    );
    reset();
}

#[test]
fn image_retained_after_adoption_does_not_retain_the_publishing_realm() {
    tests::init_v8();
    let isolate = &mut tests::new_isolate();
    v8::scope!(let scope, isolate);
    let child = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, child);
    install_independent_html(scope, "<!doctype html><body>");
    let image = eval(
        scope,
        "globalThis.retainedImage=document.createElement('img');retainedImage",
    );
    let token = child.get_security_token(scope);
    let (root_weak, document_weak) = {
        v8::scope!(let inner, scope);
        let root = v8::Context::new(inner, Default::default());
        root.set_security_token(token);
        let inner = &mut v8::ContextScope::new(inner, root);
        install_independent_html(inner, "<!doctype html><body>");
        let root_host = host(inner, root);
        let global = root.global(inner);
        for (name, value) in [
            ("imageForTest", image),
            ("imagePeerForTest", child.global(inner).into()),
            ("imageHostForTest", root_host),
        ] {
            let key = v8::String::new(inner, name).unwrap();
            global.set(inner, key.into(), value);
        }
        eval(inner,"document.body.append(imageForTest);imageHostForTest.setState(imageForTest,{complete:true,naturalWidth:3,naturalHeight:2,currentSrc:'pixel'});imagePeerForTest.document.body.append(imageForTest);'published'");
        let document = eval(inner, "document").to_object(inner).unwrap();
        (v8::Weak::new(inner, root), v8::Weak::new(inner, document))
    };
    eval(scope, "document.body.contains(retainedImage)");
    flush_ownership(scope);
    for _ in 0..3 {
        scope.low_memory_notification();
    }
    assert!(
        root_weak.is_empty(),
        "private image state retained publishing realm"
    );
    assert!(
        document_weak.is_empty(),
        "private image state retained publishing document"
    );
    assert_eq!(
        eval(scope, "retainedImage.naturalWidth").int32_value(scope),
        Some(3)
    );
    reset();
}
