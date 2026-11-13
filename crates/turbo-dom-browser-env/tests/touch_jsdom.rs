use sha2::{Digest, Sha256};
use turbo_dom_browser_env::browser_env::{
    install_independent_html, install_with_html, reset, set_media_mode, set_touch_event_mode,
    MediaMode, TouchEventMode,
};

fn eval(scope: &mut v8::PinScope, text: &str) -> String {
    let text = v8::String::new(scope, text).unwrap();
    v8::Script::compile(scope, text, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn jsdom_touch_lists_keep_entry_identity_independently_of_media() {
    assert_eq!(TouchEventMode::default(), TouchEventMode::Chrome);
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<canvas id=canvas></canvas>");
    assert_eq!(eval(scope, "(()=>{try{new TouchEvent('x',{touches:[{}]});return 'accepted'}catch(e){return e.name}})()"), "TypeError");
    set_touch_event_mode(scope, TouchEventMode::Jsdom);
    set_media_mode(scope, MediaMode::Chrome);
    let probe = include_str!("../scripts/touch-jsdom-probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("../src/touch_jsdom_jsdom27.json")).unwrap();
    assert!(control["jsdom"].as_str().unwrap().starts_with("27."));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    let observed: serde_json::Value = serde_json::from_str(&eval(
        scope,
        &format!("JSON.stringify({})", probe.trim().trim_end_matches(';')),
    ))
    .unwrap();
    assert_eq!(observed, control["result"]);
    assert_eq!(
        eval(
            scope,
            r#"(()=>{try{
      const touch={clientX:10,clientY:10};
      for(const name of ['touches','targetTouches','changedTouches']){
        const event=new TouchEvent('touchstart',{[name]:[touch]});
        const list=event[name];
        if(list.length!==1||list[0]!==touch||list.item(0)!==touch||[...list][0]!==touch||list.item(1)!==null)return name;
      }
      const values=[0,false,null,undefined,touch];
      const any=new TouchEvent('touchstart',{touches:values}).touches;
      for(let i=0;i<values.length;i++)if(any[i]!==values[i]||any.item(i)!==values[i])return 'entry changed';
      const event=new TouchEvent('touchstart',{touches:[touch]});
      let seen;const canvas=document.getElementById('canvas');
      canvas.addEventListener('touchstart',e=>seen=e.touches[0]);canvas.dispatchEvent(event);
      globalThis.retainedTouchList=event.touches;
      return seen===touch&&seen.clientX===10&&seen.clientY===10?'ok':'dispatch';
    }catch(e){return String(e)}})()"#
        ),
        "ok"
    );
    {
        let foreign = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, foreign);
        install_independent_html(scope, "<body>foreign</body>");
        assert_eq!(eval(scope, "(()=>{try{new TouchEvent('x',{touches:[{}]});return 'accepted'}catch(e){return e.name}})()"), "TypeError");
    }
    assert_eq!(
        eval(
            scope,
            "new TouchEvent('x',{touches:[{clientX:12}]}).touches.item(0).clientX"
        ),
        "12"
    );
    set_touch_event_mode(scope, TouchEventMode::Chrome);
    set_media_mode(scope, MediaMode::Jsdom);
    assert_eq!(eval(scope, "(()=>{for(const name of ['touches','targetTouches','changedTouches']){try{new TouchEvent('x',{[name]:[{}]});return name}catch(e){if(!(e instanceof TypeError))return 'wrong error'}}return retainedTouchList.item(0).clientX===10?'ok':'changed'})()"), "ok");
    reset();
}
