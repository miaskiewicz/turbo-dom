use turbo_dom_browser_env::browser_env::{install_independent_html, install_with_html, reset};

#[test]
fn touch_constructor_and_event_lists() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<button id=target>tap</button>");
    let code = v8::String::new(scope, r#"
      (() => { try {
        const target = document.getElementById('target');
        const touch = new Touch({identifier:7,target,clientX:12,pageY:15,force:0.5,radiusX:0.1});
        const event = new TouchEvent('touchstart', {touches:[touch],targetTouches:[touch],changedTouches:[touch],ctrlKey:true});
        const ok = touch.identifier === 7 && touch.target === target && touch.clientX === 12 && touch.pageY === 15 && touch.screenX === 0 && touch.force === 0.5 && touch.radiusX === Math.fround(0.1) &&
          touch instanceof Touch && event instanceof UIEvent && event.touches instanceof TouchList && event.touches.length === 1 && event.touches[0] === touch && event.touches.item(0) === touch && event.touches.item(9) === null && [...event.touches][0] === touch && event.changedTouches[0] === touch && event.targetTouches[0] === touch && event.ctrlKey;
        if (!ok) return 'incorrect fields or lists';
        if (Reflect.deleteProperty(event.touches,'0') || !Reflect.set(event.touches,'9',touch) || event.touches[9] !== undefined) return 'mutable list indices';
        for (const init of [{}, {identifier:1,target:{}}, {identifier:1,target:Object.create(EventTarget.prototype)}, {identifier:1,target,clientX:Infinity}]) { try { new Touch(init); return 'accepted invalid touch'; } catch (e) { if (!(e instanceof TypeError)) return String(e); } }
        try { new TouchEvent('x',{touches:null}); return 'accepted null list'; } catch (e) { if (!(e instanceof TypeError)) return String(e); }
        try { new TouchEvent('x',{touches:[{}]}); return 'accepted invalid list'; } catch (e) { if (!(e instanceof TypeError)) return String(e); }
        const get = Object.getOwnPropertyDescriptor(Touch.prototype,'identifier').get;
        try { get.call({}); return 'unbranded getter'; } catch (e) { if (!(e instanceof TypeError)) return String(e); }
        const saved = {Number, isFinite, Symbol, fround:Math.fround, concat:Array.prototype.concat, forEach:Array.prototype.forEach};
        let guarded;
        try {
          const poison = () => { throw new Error('guest intrinsic reached'); };
          globalThis.Number=poison;globalThis.isFinite=poison;globalThis.Symbol=poison;Math.fround=poison;Array.prototype.concat=poison;Array.prototype.forEach=poison;
          const guardedTouch = new Touch({identifier:4,target,clientX:3,radiusX:0.1});
          const guardedEvent = new TouchEvent('x',{touches:[guardedTouch]});
          const list = guardedEvent.touches;
          guarded = list.item(0) === guardedTouch && [...list][0] === guardedTouch && guardedTouch.clientX === 3;
        } finally {
          globalThis.Number=saved.Number;globalThis.isFinite=saved.isFinite;globalThis.Symbol=saved.Symbol;Math.fround=saved.fround;Array.prototype.concat=saved.concat;Array.prototype.forEach=saved.forEach;
        }
        if (!guarded) return 'hostile intrinsics changed Touch';
        let seen; target.addEventListener('touchstart', e => seen = e.changedTouches[0]); target.dispatchEvent(event);
        return seen === touch ? 'ok' : 'dispatch payload lost';
      } catch (e) { return String(e); } })()
    "#).unwrap();
    let got = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    assert_eq!(got.to_rust_string_lossy(scope), "ok");
    let foreign = v8::Context::new(scope, Default::default());
    let (touch, target) = {
        let scope = &mut v8::ContextScope::new(scope, foreign);
        install_independent_html(scope, "<button id=foreign>foreign</button>");
        let code = v8::String::new(
            scope,
            "new Touch({identifier:9,target:document.getElementById('foreign')})",
        )
        .unwrap();
        let touch = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let code = v8::String::new(scope, "new EventTarget()").unwrap();
        let target = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        (touch, target)
    };
    let key = v8::String::new(scope, "foreignTouch").unwrap();
    context.global(scope).set(scope, key.into(), touch);
    let key = v8::String::new(scope, "foreignTarget").unwrap();
    context.global(scope).set(scope, key.into(), target);
    let code = v8::String::new(scope, "Object.getOwnPropertyDescriptor(Touch.prototype,'identifier').get.call(foreignTouch) === 9 && new TouchEvent('x',{touches:[foreignTouch]}).touches[0] === foreignTouch && new Touch({identifier:3,target:foreignTarget}).target === foreignTarget").unwrap();
    let got = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    assert!(got.boolean_value(scope));
    reset();
}
