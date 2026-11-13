use turbo_dom_browser_env::browser_env::{
    install_with_html, reset, set_computed_style_mode, ComputedStyleMode,
};
#[test]
fn explicit_colors_match_declared_serialization_and_live_views_do_not_stale() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<span id=target></span><style>.colored{text-decoration:underline #161925}</style>",
    );
    set_computed_style_mode(scope, ComputedStyleMode::Jsdom);
    let text = v8::String::new(scope, r#"(()=>{try {
      const span=document.getElementById('target'); const errors=[];
      span.style.textDecoration='underline #161925';
      const live=getComputedStyle(span);
      const rgb='underline rgb(22, 25, 37)';
      if(span.style.textDecoration!==rgb)return 'unexpected declared spelling: '+span.style.textDecoration;
      if(live.textDecoration!==span.style.textDecoration)errors.push('hex mismatch: '+live.textDecoration);
      if(live.getPropertyValue('text-decoration')!==span.style.getPropertyValue('text-decoration'))errors.push('property mismatch');
      const before=span.getAttribute('style');
      span.style.textDecoration=rgb;
      if(span.getAttribute('style')!==before)return 'format-only attribute changed';
      if(live.textDecoration!==rgb||getComputedStyle(span).textDecoration!==rgb)errors.push('stale format-only value: '+live.textDecoration);
      span.style.textDecoration='underline #abcdef';
      if(live.textDecoration!=='underline rgb(171, 205, 239)')errors.push('changed color mismatch: '+live.textDecoration);
      span.style.textDecoration='line-through';
      if(live.textDecoration!=='line-through')errors.push('synthesized color');
      span.style.cssText='text-decoration:underline #161925';
      if(live.textDecoration!==span.style.textDecoration)errors.push('cssText mismatch');
      span.setAttribute('style','text-decoration:underline #abcdef');
      if(live.textDecoration!==span.style.textDecoration)errors.push('attribute mismatch');
      span.style.cssText='';span.className='colored';
      if(live.textDecoration!==document.styleSheets[0].cssRules[0].style.textDecoration)errors.push('stylesheet mismatch: '+live.textDecoration);
      return errors.length?errors.join('; '):'ok';
    } catch(e) { return String(e); }})()"#).unwrap();
    let result = v8::Script::compile(scope, text, None)
        .unwrap()
        .run(scope)
        .unwrap();
    assert_eq!(result.to_rust_string_lossy(scope), "ok");
    reset();
}
