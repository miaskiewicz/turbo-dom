use super::*;

fn run(html: &str, code: &str) -> serde_json::Value {
    run_with_font_defaults(html, code, None)
}

fn run_with_font_defaults(html: &str, code: &str, fonts: Option<(&str, f64)>) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    if let Some((family, size)) = fonts {
        assert!(set_computed_font_defaults(context, family, size));
    }
    install_with_html(scope, html);
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "document").unwrap();
    let document = global
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let record = get_interface_record(scope, document)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "cssHost").unwrap();
    let host = record.get(scope, key.into()).unwrap();
    let key = v8::String::new(scope, "__cssTestHost").unwrap();
    global.set(scope, key.into(), host);
    if code.contains("__cssTestNs") {
        let namespace = engine_namespace(scope).unwrap();
        let key = v8::String::new(scope, "__cssTestNs").unwrap();
        global.set(scope, key.into(), namespace.into());
    }
    if code.contains("__cssTestCollectionRecord") {
        fn read_collection_record<'s>(
            scope: &mut v8::PinScope<'s, '_>,
            args: v8::FunctionCallbackArguments<'s>,
            mut rv: v8::ReturnValue,
        ) {
            if let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(0)) {
                if let Some(record) = get_interface_record(scope, object) {
                    rv.set(record);
                }
            }
        }
        let getter = v8::Function::new(scope, read_collection_record).unwrap();
        let key = v8::String::new(scope, "__cssTestCollectionRecord").unwrap();
        global.set(scope, key.into(), getter.into());
    }
    if code.contains("__cssTestCascadeRuns") {
        // The cascade-run counter behind the test-only `__noteCascadeRun` hook: how many
        // fresh computed-style calculations the page has caused so far (never page-visible).
        fn cascade_runs(
            scope: &mut v8::PinScope,
            _args: v8::FunctionCallbackArguments,
            mut rv: v8::ReturnValue,
        ) {
            rv.set(v8::Number::new(scope, super::cascade_runs() as f64).into());
        }
        let getter = v8::Function::new(scope, cascade_runs).unwrap();
        let key = v8::String::new(scope, "__cssTestCascadeRuns").unwrap();
        global.set(scope, key.into(), getter.into());
    }
    if code.contains("__cssTestCssUpdates") {
        // The CSS animation / transition update passes behind the test-only
        // `__noteCssUpdatePass` hook (never page-visible).
        fn css_updates(
            scope: &mut v8::PinScope,
            _args: v8::FunctionCallbackArguments,
            mut rv: v8::ReturnValue,
        ) {
            rv.set(v8::Number::new(scope, super::css_update_passes() as f64).into());
        }
        let getter = v8::Function::new(scope, css_updates).unwrap();
        let key = v8::String::new(scope, "__cssTestCssUpdates").unwrap();
        global.set(scope, key.into(), getter.into());
    }
    if code.contains("__cssTestStyleLookups") {
        // The computed-style cache lookups behind the test-only `__noteStyleLookup` hook.
        fn style_lookups(
            scope: &mut v8::PinScope,
            _args: v8::FunctionCallbackArguments,
            mut rv: v8::ReturnValue,
        ) {
            rv.set(v8::Number::new(scope, super::style_lookups() as f64).into());
        }
        let getter = v8::Function::new(scope, style_lookups).unwrap();
        let key = v8::String::new(scope, "__cssTestStyleLookups").unwrap();
        global.set(scope, key.into(), getter.into());
    }
    if code.contains("__cssTestEntryBuilds") {
        // The cascade rule-entry list builds behind the test-only `__noteEntryBuild` hook.
        fn entry_builds(
            scope: &mut v8::PinScope,
            _args: v8::FunctionCallbackArguments,
            mut rv: v8::ReturnValue,
        ) {
            rv.set(v8::Number::new(scope, super::entry_builds() as f64).into());
        }
        let getter = v8::Function::new(scope, entry_builds).unwrap();
        let key = v8::String::new(scope, "__cssTestEntryBuilds").unwrap();
        global.set(scope, key.into(), getter.into());
    }
    if code.contains("__fontInertDoc") {
        let document = parse_html_document(scope, "<!doctype html><body><b>inert</b>", false);
        let key = v8::String::new(scope, "__fontInertDoc").unwrap();
        global.set(scope, key.into(), document.into());
    }
    let code = v8::String::new(scope, code).unwrap();
    v8::tc_scope!(let scope,scope);
    let value = v8::Script::compile(scope, code, None)
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .run(scope)
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)));
    let value = value.to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&value).unwrap()
}

#[test]
fn native_style_text_ignores_own_getters_and_preserves_sheet_identity_until_actual_mutation() {
    let value = run(
        "<style id=s>.real {color:red}</style>",
        r#"(()=>{
      const style=document.getElementById('s');let reads=0;
      Object.defineProperty(style,'textContent',{configurable:true,get(){reads++;return '.fake {color:blue}';}});
      const original=style.sheet;
      original.insertRule('.inserted {width:20px}',1);
      const retained=style.sheet===original;
      const first=original.cssRules[0].selectorText;
      const setter=Object.getOwnPropertyDescriptor(Node.prototype,'textContent').set;
      setter.call(style,'.updated {height:30px}');
      const changed=style.sheet!==original;
      return JSON.stringify({reads,retained,first,changed,current:style.sheet.cssRules[0].selectorText,oldCount:original.cssRules.length});
    })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"reads":0,"retained":true,"first":".real","changed":true,"current":".updated","oldCount":2})
    );
}

#[test]
fn private_css_export_reads_real_mutations_disabled_and_media_without_public_hooks() {
    let value = run(
        "<style id=s media=screen>.real {color:red}</style>",
        r#"(()=>{
      const style=document.getElementById('s'),sheet=style.sheet,rule=sheet.cssRules[0];
      rule.style.setProperty('width','27px','important');sheet.disabled=true;
      let reads=0;
      for(const [object,key] of [[style,'textContent'],[style,'getAttribute'],[sheet,'cssRules'],[sheet,'disabled'],[sheet,'media'],[rule,'cssText'],[rule,'style']])Object.defineProperty(object,key,{configurable:true,get(){reads++;throw Error('public hook');}});
      const savedMap=Map.prototype.get,savedArray=Array.prototype.map,savedIterator=Array.prototype[Symbol.iterator];
      Map.prototype.get=Array.prototype.map=Array.prototype[Symbol.iterator]=function(){reads++;throw Error('prototype hook');};
      Object.defineProperty(Object.prototype,'cssRules',{configurable:true,get(){reads++;throw Error('inherited public hook');}});
      let exported;try{exported=__cssTestHost.exportSheet(sheet);}finally{delete Object.prototype.cssRules;Map.prototype.get=savedMap;Array.prototype.map=savedArray;Array.prototype[Symbol.iterator]=savedIterator;}
      return JSON.stringify({reads,exported,nullArrays:Object.getPrototypeOf(exported.rules)===null,nullRecord:Object.getPrototypeOf(exported)===null});
    })()"#,
    );
    assert_eq!(value["reads"], 0);
    assert_eq!(value["nullArrays"], true);
    assert_eq!(value["nullRecord"], true);
    assert_eq!(value["exported"]["disabled"], true);
    assert_eq!(value["exported"]["media"], "screen");
    assert_eq!(
        value["exported"]["rules"][0]["cssText"],
        ".real { color: red; width: 27px !important; }"
    );
}

#[test]
fn private_css_export_retains_group_changes_import_ownership_and_cross_origin_policy() {
    let value = run("<style id=s>@import url('child.css') screen; @media screen { .inside {color:red} }</style>",r#"(()=>{
      const parent=document.getElementById('s').sheet,imported=parent.cssRules[0],group=parent.cssRules[1];
      group.cssRules[0].style.color='blue';
      const child=new CSSStyleSheet();__cssTestHost.attachImport(imported,child,'https://other.test/child.css','.child {width:33px}',false);
      let denied='';try{void child.cssRules;}catch(error){denied=error.name;}
      let hooks=0;Object.defineProperty(group,'cssRules',{get(){hooks++;throw Error('public grouping hook');}});
      const exported=__cssTestHost.exportSheet(parent);
      return JSON.stringify({hooks,denied,exported});
    })()"#);
    assert_eq!(value["hooks"], 0);
    assert_eq!(value["denied"], "SecurityError");
    let rules = &value["exported"]["rules"];
    assert_eq!(rules[0]["styleSheet"]["originClean"], false);
    assert_eq!(
        rules[0]["styleSheet"]["href"],
        "https://other.test/child.css"
    );
    assert_eq!(
        rules[0]["styleSheet"]["rules"][0]["cssText"],
        ".child { width: 33px; }"
    );
    assert_eq!(rules[1]["prelude"], "@media screen");
    assert_eq!(rules[1]["rules"][0]["cssText"], ".inside { color: blue; }");
    assert_eq!(
        rules[1]["cssText"],
        "@media screen { .inside { color: blue; } }"
    );
}

#[test]
fn private_css_export_observes_native_media_attributes_and_bounded_cycles() {
    let value = run(
        "<style id=s media=print>@import url('child.css'); .root {height:8px}</style>",
        r#"(()=>{
      const style=document.getElementById('s'),sheet=style.sheet;
      style.setAttribute('media','screen and (min-width: 10px)');
      let hooks=0;Object.defineProperty(style,'getAttribute',{value(){hooks++;throw Error('public attribute hook');}});
      __cssTestHost.attachImport(sheet.cssRules[0],sheet,'https://layout.test/root.css',undefined,true);
      const exported=__cssTestHost.exportSheet(sheet);
      let forged='';try{__cssTestHost.exportSheet({});}catch(error){forged=error.name;}
      return JSON.stringify({hooks,forged,exported});
    })()"#,
    );
    assert_eq!(value["hooks"], 0);
    assert_eq!(value["forged"], "TypeError");
    assert_eq!(value["exported"]["media"], "screen and (min-width: 10px)");
    assert_eq!(value["exported"]["rules"][0]["styleSheet"]["cycle"], true);
}

#[test]
fn private_css_export_retains_font_face_declaration_changes() {
    let value = run(
        "<style id=s>@font-face { font-family: test; src: url(font.woff2); }</style>",
        r#"(()=>{
      const sheet=document.getElementById('s').sheet,rule=sheet.cssRules[0];
      rule.style.setProperty('font-family','updated');
      let reads=0;Object.defineProperty(rule,'style',{get(){reads++;throw Error('public font hook');}});
      const exported=__cssTestHost.exportSheet(sheet);
      return JSON.stringify({reads,text:exported.rules[0].cssText});
    })()"#,
    );
    assert_eq!(value["reads"], 0);
    assert_eq!(
        value["text"],
        "@font-face { font-family: updated; src: url(\"font.woff2\"); }"
    );
}

#[test]
fn public_live_css_behavior_matches_independent_chrome_control() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("css_live_chrome155.json")).unwrap();
    let probe = include_str!("css_live_probe.js")
        .trim()
        .trim_end_matches(';');
    assert_eq!(
        run(
            "<!doctype html><html><head></head><body></body></html>",
            &format!("JSON.stringify({probe})")
        ),
        expected["result"]
    );
}

#[test]
fn private_css_export_enforces_its_actual_serialized_text_budget() {
    let value = run(
        "<style id=s>@media screen { .inside {color:red} }</style>",
        r#"(()=>{
      const sheet=document.getElementById('s').sheet;
      sheet.cssRules[0].cssRules[0].style.setProperty('--large','x'.repeat(1100000));
      let failure='';try{__cssTestHost.exportSheet(sheet);}catch(error){failure=error.name+':'+error.message;}
      return JSON.stringify(failure);
    })()"#,
    );
    assert_eq!(value, "RangeError:Live stylesheet text budget exceeded");
}

#[test]
fn native_computed_color_resolves_initial_inherited_keywords_and_live_declarations() {
    let actual = run(
        "<!doctype html><body></body>",
        &format!(
            "JSON.stringify({})",
            include_str!("computed_color_probe.js")
                .trim()
                .trim_end_matches(';')
        ),
    );
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("computed_color_chrome155.json")).unwrap();
    assert_eq!(actual, expected["result"]);
}

#[test]
fn native_computed_color_keeps_linked_sheet_values_and_inherits_without_guest_global_reads() {
    let actual=run("<!doctype html><head><link id=link rel=stylesheet href=/authored.css></head><body><div id=parent><span id=child></span></div></body>",r#"(()=>{
      __cssTestHost.attachLinked(document.getElementById('link'),'#parent { color: green }','http://localhost/authored.css',false);
      const parent=document.getElementById('parent'),child=document.getElementById('child');
      const read=getComputedStyle;globalThis.getComputedStyle=()=>{throw Error('guest global must not be consulted')};
      return JSON.stringify({parent:read(parent).color,child:read(child).color,private:typeof __normalizeComputedColor});
    })()"#);
    assert_eq!(
        actual,
        serde_json::json!({"parent":"rgb(0, 128, 0)","child":"rgb(0, 128, 0)","private":"undefined"})
    );
}

#[test]
fn quoted_selector_delimiters_and_comment_text_survive_cssom_roundtrips() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("css_selector_lexemes_chrome155.json")).unwrap();
    let probe = include_str!("css_selector_lexemes_probe.js")
        .trim()
        .trim_end_matches(';');
    assert_eq!(
        run(
            "<!doctype html><html><head></head><body></body></html>",
            &format!("JSON.stringify({probe})")
        ),
        expected["result"]
    );
}

#[test]
fn typed_descriptor_blocks_match_independent_chrome_cssom() {
    use sha2::{Digest, Sha256};
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("css_descriptor_chrome155.json")).unwrap();
    let probe = include_str!("css_descriptor_probe.js");
    assert_eq!(expected["errors"], serde_json::json!([]));
    assert_eq!(expected["browser"], "Chrome/155.0.8059.80");
    assert_eq!(
        expected["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        expected["casesSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("css_descriptor_cases.json"))
        )
    );
    assert_eq!(
        run(
            "<!doctype html><head></head><body></body>",
            &format!("JSON.stringify({})", probe.trim().trim_end_matches(';'))
        ),
        expected["result"][0]["result"]
    );
}

#[test]
fn private_css_export_retains_descriptor_text_without_fabricated_child_rules() {
    let value = run(
        "<!doctype html><head></head><body></body>",
        r#"(()=>{
      const sheet=new CSSStyleSheet();sheet.replaceSync('@property --paint {syntax:"<color>";initial-value:transparent;inherits:false} @view-transition {navigation:auto}');
      const rules=sheet.cssRules,publicText=Array.from(rules,rule=>rule.cssText);let reads=0;
      const children=Array.from(rules,rule=>'cssRules' in rule);
      for(let i=0;i<rules.length;i++)for(const key of ['cssText','cssRules'])Object.defineProperty(rules[i],key,{configurable:true,get(){reads++;throw Error('public rule hook');}});
      const exported=__cssTestHost.exportSheet(sheet);const text=[],nested=[];
      for(let i=0;i<exported.rules.length;i++){text.push(exported.rules[i].cssText);nested.push(Object.hasOwn(exported.rules[i],'rules'));}
      return JSON.stringify({reads,children,nested,equal:JSON.stringify(text)===JSON.stringify(publicText),text});
    })()"#,
    );
    assert_eq!(value["reads"], 0);
    assert_eq!(value["children"], serde_json::json!([false, false]));
    assert_eq!(value["nested"], serde_json::json!([false, false]));
    assert_eq!(value["equal"], true);
    assert!(value["text"][0]
        .as_str()
        .unwrap()
        .contains("initial-value: transparent"));
    assert!(value["text"][1]
        .as_str()
        .unwrap()
        .contains("navigation: auto"));
}

#[test]
fn computed_style_uses_private_native_matcher_and_owner_document() {
    use sha2::{Digest, Sha256};
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("computed_intrinsics_chrome155.json")).unwrap();
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("computed_intrinsics_cases.json")).unwrap();
    let probe = include_str!("computed_intrinsics_probe.js");
    assert_eq!(expected["errors"], serde_json::json!([]));
    assert_eq!(expected["browser"], "Chrome/155.0.8059.80");
    assert_eq!(
        expected["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        expected["casesSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("computed_intrinsics_cases.json"))
        )
    );
    assert_eq!(
        run(
            cases[0]["html"].as_str().unwrap(),
            &format!("JSON.stringify({})", probe.trim().trim_end_matches(';'))
        ),
        expected["result"][0]["result"]
    );
}

#[test]
fn inherited_computed_values_use_linear_stylesheet_scans_and_keep_live_results() {
    let value = run(
        "<!doctype html><head><style>.unused {color:red}</style></head><body>",
        r#"(()=>{
          const read=getComputedStyle,query=document.querySelectorAll,rows=[];
          let scans=0;
          document.querySelectorAll=function(selector){if(selector==='style,link')scans++;return Reflect.apply(query,this,arguments);};
          try {
            for(const percentage of [false,true])for(const depth of [3,5,7]){
              const root=document.createElement('section');root.style.cssText='color:red;width:256px;height:512px';document.body.appendChild(root);
              let leaf=root;
              for(let index=0;index<depth;index++){const child=document.createElement('div');child.style.cssText='color:inherit'+(percentage?';width:50%;height:50%':'');leaf.appendChild(child);leaf=child;}
              scans=0;const retained=read(leaf),color=retained.getPropertyValue('color');
              const width=percentage?retained.getPropertyValue('width'):null,height=percentage?retained.getPropertyValue('height'):null;
              rows.push({depth,percentage,scans,color,width,height});
              root.remove();
            }
          } finally {document.querySelectorAll=query;}
          return JSON.stringify(rows);
        })()"#,
    );
    for row in value.as_array().unwrap() {
        let depth = row["depth"].as_u64().unwrap();
        assert!(
            row["scans"].as_u64().unwrap() <= 8 * (depth + 2),
            "ancestor computation rescanned exponentially: {row}"
        );
        assert_eq!(row["color"], "rgb(255, 0, 0)", "{row}");
        if row["percentage"].as_bool().unwrap() {
            assert_eq!(row["width"], format!("{}px", 256_u64 >> depth), "{row}");
            assert_eq!(row["height"], format!("{}px", 512_u64 >> depth), "{row}");
        }
    }
}

#[test]
fn computed_ancestor_snapshots_keep_auto_percentage_paths_linear() {
    let value = run(
        "<!doctype html><head><style>.unused {color:red}</style></head><body>",
        r#"(()=>{
          const query=document.querySelectorAll,rows=[];let scans=0;
          document.querySelectorAll=function(selector){if(selector==='style,link')scans++;return Reflect.apply(query,this,arguments);};
          try {
            for(const mode of ['auto','alternating'])for(const depth of [3,5,7]){
              const root=document.createElement('section');root.style.cssText='color:red;width:256px;height:512px;font:20px/2 serif';document.body.appendChild(root);
              let leaf=root,halves=0;
              for(let index=0;index<depth;index++){const child=document.createElement('div'),percentage=mode==='alternating'&&index%2===0;child.style.cssText='color:inherit;width:'+(percentage?'50%':'auto')+';height:auto';if(percentage)halves++;leaf.appendChild(child);leaf=child;}
              const target=document.createElement('span');target.style.cssText='width:50%;height:50%;font-size:50%';leaf.appendChild(target);halves++;
              scans=0;const retained=getComputedStyle(target),width=retained.width,height=retained.height,font=retained.fontSize;
              rows.push({mode,depth,scans,width,height,font,expectedWidth:256/2**halves});root.remove();
            }
          } finally {document.querySelectorAll=query;}
          return JSON.stringify(rows);
        })()"#,
    );
    for row in value.as_array().unwrap() {
        let depth = row["depth"].as_u64().unwrap();
        assert!(
            row["scans"].as_u64().unwrap() <= 4 * (depth + 4),
            "a public read revisited resolved ancestors: {row}"
        );
        assert_eq!(row["width"], format!("{}px", row["expectedWidth"]), "{row}");
        assert_eq!(row["font"], "10px", "{row}");
    }
}

#[test]
fn computed_ancestor_snapshots_expire_before_later_public_reads() {
    let value = run(
        "<!doctype html><head><style id=s>.host {color:red;width:256px;font:20px/2 serif}</style></head><body><section id=a class=host><div style='color:inherit;width:auto'><span id=t style='color:inherit;width:50%;font-size:50%'></span></div></section><section id=b style='color:blue;width:512px;font:30px/3 serif'></section>",
        r#"(()=>{
          const target=document.getElementById('t'),first=document.getElementById('a'),second=document.getElementById('b'),rule=document.getElementById('s').sheet.cssRules[0],retained=getComputedStyle(target),rows=[];
          const capture=()=>rows.push([retained.color,retained.width,retained.fontSize,retained.lineHeight]);
          capture();rule.style.width='128px';rule.style.color='green';capture();
          second.appendChild(target.parentElement);capture();second.style.fontSize='40px';capture();
          target.parentElement.remove();rows.push([retained.color,retained.fontSize]);first.appendChild(target);capture();
          return JSON.stringify(rows);
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["rgb(255, 0, 0)", "128px", "10px", "20px"],
            ["rgb(0, 128, 0)", "64px", "10px", "20px"],
            ["rgb(0, 0, 255)", "256px", "15px", "45px"],
            ["rgb(0, 0, 255)", "256px", "20px", "60px"],
            ["", ""],
            ["rgb(0, 128, 0)", "64px", "10px", "20px"]
        ])
    );
}

#[test]
fn computed_ancestor_snapshot_cache_ignores_public_weakmap_hooks() {
    let value = run(
        "<!doctype html><body><section style='color:red;width:256px;font:20px serif'><div style='width:auto'><span id=t style='width:50%;font-size:50%'></span></div></section>",
        r#"(()=>{
          const target=document.getElementById('t'),OriginalWeakMap=WeakMap,prototype=WeakMap.prototype,get=prototype.get,set=prototype.set;let hooks=0,result;
          const poisoned=()=>{hooks++;throw Error('public WeakMap hook');};
          globalThis.WeakMap=poisoned;prototype.get=prototype.set=poisoned;
          try {const style=getComputedStyle(target);result=[style.color,style.width,style.fontSize];}
          finally {globalThis.WeakMap=OriginalWeakMap;prototype.get=get;prototype.set=set;}
          return JSON.stringify({result,hooks});
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"result":["rgb(255, 0, 0)","128px","10px"],"hooks":0})
    );
}

#[test]
fn computed_style_matches_private_selector_lists_before_declaration_work() {
    let value = run(
        "<!doctype html><head><style id=s>.target{width:5px;color:green}</style></head><body><div id=t class=target data-token='{,}'></div><div id=other></div>",
        r#"(()=>{
          const target=document.getElementById('t'),other=document.getElementById('other'),sheet=document.getElementById('s').sheet;
          for(let index=0;index<24;index++)sheet.insertRule('.never-'+index+'{width:99px;color:red;--unused:'+index+'}',sheet.cssRules.length);
          sheet.insertRule('.target{width:11px;color:blue}',sheet.cssRules.length);
          const matched=sheet.cssRules[sheet.cssRules.length-1],matchedStyle=matched.style,selectorSetter=Object.getOwnPropertyDescriptor(CSSStyleRule.prototype,'selectorText').set,unmatched=Array.from(sheet.cssRules).slice(1,-1);
          let declarationReads=0,publicReads=0;
          for(const rule of unmatched){
            const record=__cssTestCollectionRecord(rule.style).css.declaration,names=record.names;
            record.names=new Proxy(names,{get(target,key,receiver){declarationReads++;return Reflect.get(target,key,receiver);}});
          }
          for(const rule of [...unmatched,matched])for(const key of ['cssText','selectorText','style'])Object.defineProperty(rule,key,{configurable:true,get(){publicReads++;throw Error('public rule getter');}});
          Object.defineProperty(target,'matches',{configurable:true,get(){publicReads++;throw Error('public matcher getter');}});
          const saved=Object.getOwnPropertyDescriptor(Element.prototype,'matches');
          Object.defineProperty(Element.prototype,'matches',{...saved,value(){publicReads++;throw Error('public prototype matcher');}});
          const retained=getComputedStyle(target),rows=[];
          try {
            rows.push([retained.width,retained.color]);
            matchedStyle.width='17px';rows.push([retained.width,retained.color]);
            selectorSetter.call(matched,'.never');rows.push([retained.width,retained.color]);
          }finally{Object.defineProperty(Element.prototype,'matches',saved);}
          return JSON.stringify({rows,declarationReads,publicReads,otherUntouched:other.id==='other'});
        })()"#,
    );
    assert_eq!(
        value["publicReads"], 0,
        "private style computation used public hooks: {value}"
    );
    assert_eq!(
        value["rows"],
        serde_json::json!([
            ["11px", "rgb(0, 0, 255)"],
            ["17px", "rgb(0, 0, 255)"],
            ["5px", "rgb(0, 128, 0)"]
        ])
    );
    assert_eq!(value["otherUntouched"], true);
    assert_eq!(
        value["declarationReads"], 0,
        "unmatched declarations were serialized: {value}"
    );
}

#[test]
fn computed_selector_lists_preserve_quoted_delimiters_and_live_invalid_assignment() {
    let value=run(
        "<!doctype html><head><style id=s>.target{width:7px}[data-token='{,}'] {width:13px} .target, [data-other='a,b'] {height:19px}</style></head><body><div id=t class=target data-token='{,}'></div><div id=other data-other='a,b'></div>",
        r#"(()=>{
          const target=document.getElementById('t'),other=document.getElementById('other'),sheet=document.getElementById('s').sheet,rule=sheet.cssRules[1],style=getComputedStyle(target),rows=[];
          const read=()=>rows.push([style.width,style.height,getComputedStyle(other).height]);
          read();rule.style.width='23px';read();
          rule.selectorText="[data-token='missing,{']";read();
          rule.selectorText="[data-token='{,}']";read();
          rule.selectorText='.target, ???';read();
          const invalid=[];
          for(const selector of ['.target, ???']){
            try{sheet.insertRule(selector+' {width:99px}',sheet.cssRules.length);invalid.push('accepted');}catch(error){invalid.push(error.name);}
          }
          return JSON.stringify({rows,invalid,ruleCount:sheet.cssRules.length,selectorStillMatches:target.matches(rule.selectorText)});
        })()"#,
    );
    assert_eq!(
        value["rows"],
        serde_json::json!([
            ["13px", "19px", "19px"],
            ["23px", "19px", "19px"],
            ["7px", "19px", "19px"],
            ["23px", "19px", "19px"],
            ["23px", "19px", "19px"]
        ]),
        "quoted selector list behavior diverged: {value}"
    );
    assert_eq!(
        value["invalid"],
        serde_json::json!(["SyntaxError"]),
        "invalid selector list admission diverged: {value}"
    );
    assert_eq!(value["ruleCount"], 3);
    assert_eq!(value["selectorStillMatches"], true);
}

#[test]
fn cssom_unknown_pseudo_selector_admission_matches_chrome155() {
    let value=run(
        "<!doctype html><head><style id=s>.target{width:7px}[data-token='{,}'] {width:13px} .target, [data-other='a,b'] {height:19px}</style></head><body><div id=t class=target data-token='{,}'></div><div id=other data-other='a,b'></div>",
        r#"(()=>{
          const target=document.getElementById('t'),other=document.getElementById('other'),sheet=document.getElementById('s').sheet,rule=sheet.cssRules[1],style=getComputedStyle(target),rows=[];
          const read=()=>rows.push([style.width,style.height,getComputedStyle(other).height]);
          read();rule.style.width='23px';read();
          rule.selectorText="[data-token='missing,{']";read();
          rule.selectorText="[data-token='{,}']";read();
          rule.selectorText='.target, ???';read();
          const invalid=[];
          for(const selector of ['.target, ???','.target, :definitely-unknown']){
            try{sheet.insertRule(selector+' {width:99px}',sheet.cssRules.length);invalid.push('accepted');}catch(error){invalid.push(error.name);}
          }
          return JSON.stringify({rows,invalid,ruleCount:sheet.cssRules.length,selectorStillMatches:target.matches(rule.selectorText)});
        })()"#,
    );
    assert_eq!(
        value["rows"],
        serde_json::json!([
            ["13px", "19px", "19px"],
            ["23px", "19px", "19px"],
            ["7px", "19px", "19px"],
            ["23px", "19px", "19px"],
            ["23px", "19px", "19px"]
        ]),
        "quoted selector list behavior diverged: {value}"
    );
    assert_eq!(
        value["invalid"],
        serde_json::json!(["SyntaxError", "SyntaxError"]),
        "invalid selector list admission diverged: {value}"
    );
    assert_eq!(value["ruleCount"], 3);
    assert_eq!(value["selectorStillMatches"], true);
}

#[test]
fn computed_import_rules_keep_source_order_and_live_declarations_after_flattening() {
    let value = run(
        "<!doctype html><head><style id=s>@import url('child.css'); .target {height:17px}</style></head><body><div id=t class=target></div></body>",
        r#"(()=>{
          const sheet=document.getElementById('s').sheet,child=new CSSStyleSheet(),nested=new CSSStyleSheet();
          __cssTestHost.attachImport(sheet.cssRules[0],child,'http://localhost/child.css',"@import url('nested.css'); .target {color:red;width:11px}",true);
          __cssTestHost.attachImport(child.cssRules[0],nested,'http://localhost/nested.css','.target {color:blue;width:7px;height:13px}',true);
          const retained=getComputedStyle(document.getElementById('t')),states=[];
          const capture=()=>states.push([retained.color,retained.width,retained.height]);
          capture();
          child.cssRules[1].style.color='green';nested.cssRules[0].style.height='23px';capture();
          sheet.deleteRule(1);capture();
          child.insertRule('.target {width:29px}',child.cssRules.length);capture();
          child.deleteRule(child.cssRules.length-1);capture();
          const old=child.cssRules[1];child.deleteRule(1);old.style.color='purple';capture();
          nested.cssRules[0].style.color='orange';capture();
          return JSON.stringify(states);
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["rgb(255, 0, 0)", "11px", "17px"],
            ["rgb(0, 128, 0)", "11px", "17px"],
            ["rgb(0, 128, 0)", "11px", "23px"],
            ["rgb(0, 128, 0)", "29px", "23px"],
            ["rgb(0, 128, 0)", "11px", "23px"],
            ["rgb(0, 0, 255)", "7px", "23px"],
            ["rgb(255, 165, 0)", "7px", "23px"]
        ])
    );
}

#[test]
fn computed_font_values_match_source_bound_chrome155_inheritance_and_lifecycle() {
    use sha2::{Digest, Sha256};
    let source = include_str!("computed_intrinsics_font_probe.js");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("computed_intrinsics_font_chrome155.json")).unwrap();
    assert_eq!(
        fixture["probeSha256"],
        format!("{:x}", Sha256::digest(source.as_bytes()))
    );
    assert_eq!(fixture["errors"], serde_json::json!([]));
    // Standalone V8 has no host DOMParser installer; provide the same native inert
    // document factory used by production, leaving the Chrome control unchanged.
    let adapted = source.replace("new DOMParser().parseFromString(\n\t\t\"<!doctype html><body><b>inert</b>\",\n\t\t\"text/html\",\n\t)", "__fontInertDoc");
    assert_ne!(adapted, source);
    let result = run_with_font_defaults(
        "<!doctype html><html><head></head><body></body></html>",
        &format!("JSON.stringify({})", adapted.trim().trim_end_matches(';')),
        Some(("Times", 16.0)),
    );
    for (index, row) in fixture["result"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(result["rows"][index], *row, "font row {}", row["name"]);
    }
    assert_eq!(result, fixture["result"]);
}

#[test]
fn button_ua_computed_values_match_chrome_and_remain_below_author_styles() {
    use sha2::{Digest, Sha256};
    let source = include_str!("computed_button_ua_probe.js");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("computed_button_ua_chrome155.json")).unwrap();
    assert_eq!(
        fixture["probeSha256"],
        format!("{:x}", Sha256::digest(source.as_bytes()))
    );
    assert_eq!(fixture["errors"], serde_json::json!([]));
    let actual = run_with_font_defaults(
        "<!doctype html><html><head></head><body></body></html>",
        &format!("JSON.stringify({})", source.trim().trim_end_matches(';')),
        Some(("Times", 16.0)),
    );
    for (index, row) in fixture["result"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(actual["rows"][index], *row, "button UA row {}", row["name"]);
    }
    assert_eq!(actual, fixture["result"]);
}

#[test]
fn computed_font_host_preferences_are_private_and_used_by_css_initial_values() {
    let result = run_with_font_defaults("<!doctype html><body><div id=parent style='font:italic 30px/2 serif'><span id=child style='font:initial'></span></div>", r#"JSON.stringify((()=>{
        const style=getComputedStyle(document.getElementById('child'));
        return [style.font,style.fontSize,style.lineHeight,typeof globalThis.__computedFontContext];
    })())"#, Some(("monospace", 18.0)));
    assert_eq!(
        result,
        serde_json::json!(["18px monospace", "18px", "normal", "undefined"])
    );
}

#[test]
fn computed_font_context_follows_actual_owner_preferences_adoption_and_retirement() {
    fn evaluate(scope: &mut v8::PinScope, source: &str) -> serde_json::Value {
        v8::tc_scope!(let scope, scope);
        let source = v8::String::new(scope, source).unwrap();
        let value = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| {
                panic!(
                    "{:?}",
                    scope
                        .exception()
                        .map(|value| value.to_rust_string_lossy(scope))
                )
            });
        serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap()
    }
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let parent = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, parent);
    assert!(set_computed_font_defaults(parent, "serif", 24.0));
    assert!(!set_computed_font_defaults(parent, "", 16.0));
    assert!(!set_computed_font_defaults(parent, "monospace", f64::NAN));
    assert!(!set_computed_font_defaults(parent, "monospace", 0.0));
    install_with_html(scope, "<!doctype html><body></body>");
    let child = v8::Context::new(scope, Default::default());
    child.set_security_token(parent.get_security_token(scope));
    {
        let scope = &mut v8::ContextScope::new(scope, child);
        assert!(set_computed_font_defaults(child, "monospace", 18.0));
        install_independent_html(
            scope,
            "<!doctype html><body><span id=leaf style='font-size:10px'>child</span></body>",
        );
    }
    let child_global = child.global(scope);
    let parent_global = parent.global(scope);
    set_prop(scope, parent_global, "fontChild", child_global.into());
    let initial = evaluate(
        scope,
        r#"JSON.stringify((()=>{
        const doc=fontChild.document,leaf=doc.getElementById('leaf');
        const preference=getComputedStyle(doc.body).font;
        document.documentElement.style.lineHeight='2';doc.documentElement.style.lineHeight='1.5';
        const read=style=>[style.font,style.fontSize,style.lineHeight];
        const retained=getComputedStyle(leaf);globalThis.fontRetiredStyle=fontChild.getComputedStyle(doc.body);
        const before={borrowed:read(retained),own:read(fontChild.getComputedStyle(leaf))};
        document.body.append(document.adoptNode(leaf));
        return {preference,before,adopted:{retained:read(retained),own:read(getComputedStyle(leaf))}};
    })())"#,
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "computed_intrinsics_font_realms_chrome155.json"
    ))
    .unwrap();
    use sha2::{Digest, Sha256};
    assert_eq!(fixture["errors"], serde_json::json!([]));
    assert_eq!(
        fixture["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("computed_intrinsics_font_realms_probe.js"))
        )
    );
    assert_eq!(initial["preference"], "18px monospace");
    assert_eq!(initial["before"], fixture["result"]["before"]);
    assert_eq!(initial["adopted"], fixture["result"]["adopted"]);
    let document_key = v8::String::new(scope, "document").unwrap();
    let document = child_global
        .get(scope, document_key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    {
        let _owner = DomScope::for_object(scope, document);
        // The production frame retirement boundary changes this same private flag.
        DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().browsing_context = false);
    }
    let retired = evaluate(scope, "JSON.stringify([fontRetiredStyle.font,fontRetiredStyle.lineHeight,getComputedStyle(fontChild.document.body).font])");
    assert_eq!(retired, serde_json::json!(["", "", ""]));
    reset();
}

#[test]
fn computed_fonts_preserve_unresolved_metrics_without_fabricating_inherited_values() {
    let value = run_with_font_defaults("<!doctype html><body><div id=p style='font-size:2ch;line-height:calc(1em + 1ch)'><span id=c><b id=g></b></span></div></body>", r#"JSON.stringify((()=>{
        const values=[];
        for(const id of ['p','c','g']){const style=getComputedStyle(document.getElementById(id));values.push([style.fontSize,style.lineHeight,style.font]);}
        return values;
    })())"#, Some(("serif",16.0)));
    assert_eq!(
        value,
        serde_json::json!([
            ["2ch", "calc(1em + 1ch)", "2ch / calc(1em + 1ch) serif"],
            ["2ch", "calc(1em + 1ch)", "2ch / calc(1em + 1ch) serif"],
            ["2ch", "calc(1em + 1ch)", "2ch / calc(1em + 1ch) serif"]
        ])
    );
}

#[test]
fn computed_fonts_ignore_public_slot_and_inherited_private_state_getters() {
    let value = run_with_font_defaults(
        "<!doctype html><body></body>",
        r#"JSON.stringify((()=>{
        const host=document.createElement('div'),leaf=document.createElement('span');
        const shadow=host.attachShadow({mode:'closed',slotAssignment:'manual'}),slot=document.createElement('slot');
        slot.style.cssText='font-size:30px;line-height:2';shadow.append(slot);host.append(leaf);document.body.append(host);slot.assign(leaf);
        let reads=0;Object.defineProperty(leaf,'assignedSlot',{get(){reads++;throw Error('guest slot');}});
        Object.defineProperty(Object.prototype,'rootSize',{configurable:true,get(){reads++;throw Error('guest state');}});
        let result;try{const style=getComputedStyle(leaf);result=[style.font,style.lineHeight];}finally{delete Object.prototype.rootSize;}
        return [result,reads];
    })())"#,
        Some(("serif", 16.0)),
    );
    assert_eq!(value, serde_json::json!([["30px / 60px serif", "60px"], 0]));
}

#[test]
fn computed_font_numeric_operations_ignore_guest_precision_and_unknown_unit_hooks() {
    let value = run_with_font_defaults("<!doctype html><body><span id=known style='font-size:calc(10px + 2px);line-height:1.5'></span><span id=unknown style='font-size:2ch'></span></body>", r#"JSON.stringify((()=>{
        const savedPrecision=Number.prototype.toPrecision,savedFinite=Number.isFinite;let reads=0;
        Number.prototype.toPrecision=Number.isFinite=function(){reads++;throw Error('guest numeric hook');};
        Object.defineProperty(Object.prototype,'2ch',{configurable:true,get(){reads++;throw Error('guest unit');}});
        let values;try{values=[getComputedStyle(document.getElementById('known')).font,getComputedStyle(document.getElementById('unknown')).font];}
        finally{Number.prototype.toPrecision=savedPrecision;Number.isFinite=savedFinite;delete Object.prototype['2ch'];}
        return [values,reads];
    })())"#,Some(("serif",16.0)));
    assert_eq!(
        value,
        serde_json::json!([["12px / 18px serif", "2ch serif"], 0])
    );
}

#[test]
fn font_variant_caps_subset_preserves_parser_validation_and_reset_order() {
    assert!(cssom_parser::is_supported_property("font-variant"));
    for value in [
        "normal",
        "small-caps",
        "SMALL-CAPS",
        "initial",
        "inherit",
        "unset",
        "revert",
        "revert-layer",
    ] {
        assert!(
            cssom_parser::parse(&format!("x {{font-variant:{value}}}")).contains("font-variant:"),
            "{value}"
        );
    }
    for value in ["garbage", "small-caps garbage", "123", "small-caps,normal"] {
        assert!(
            !cssom_parser::parse(&format!("x {{font-variant:{value}}}")).contains("font-variant:"),
            "{value}"
        );
    }
    let actual=run_with_font_defaults("<!doctype html><body><span id=x style='font:20px serif;font-variant:small-caps'></span></body>", r#"JSON.stringify((()=>{
        const x=document.getElementById('x'),style=getComputedStyle(x),values=[style.font];
        x.style.cssText='font-variant:small-caps;font:20px serif';values.push(style.font);
        x.style.cssText='font:20px serif;font-variant:SMALL-CAPS';values.push(style.font);return values;
    })())"#,Some(("serif",16.0)));
    assert_eq!(
        actual,
        serde_json::json!([
            "small-caps 20px serif",
            "20px serif",
            "small-caps 20px serif"
        ])
    );
}

#[test]
fn computed_font_case_normalization_uses_private_intrinsics_and_preserves_author_names() {
    let value=run_with_font_defaults("<!doctype html><body><div style='font:ITALIC BOLD 20PX/2 SERIF'><span id=x style='font-size:2EM;line-height:NORMAL;font-family:&quot;MiXeD Family&quot;,mOnOsPaCe'></span></div></body>", r#"JSON.stringify((()=>{
        const original=String.prototype.toLowerCase;let hooks=0;
        String.prototype.toLowerCase=function(){const text=String(this);if(/(?:EM|PX|NORMAL|ITALIC|BOLD|SERIF|MiXeD|mOnOsPaCe)/.test(text)){hooks++;throw Error('guest font lowercase');}return Reflect.apply(original,this,[]);};
        let result;try{const style=getComputedStyle(document.getElementById('x'));result=[style.font,style.lineHeight];}finally{String.prototype.toLowerCase=original;}
        return [result,hooks];
    })())"#,Some(("serif",16.0)));
    assert_eq!(
        value,
        serde_json::json!([["italic 700 40px \"MiXeD Family\", monospace", "normal"], 0])
    );
    let unresolved=run_with_font_defaults("<!doctype html><body><div style='font-size:var(--FontSize);font-family:var(--FontFamily);line-height:var(--LineHeight)'><span id=x></span></div></body>",r#"JSON.stringify((()=>{const style=getComputedStyle(document.getElementById('x'));return [style.fontSize,style.fontFamily,style.lineHeight];})())"#,Some(("serif",16.0)));
    assert_eq!(
        unresolved,
        serde_json::json!(["var(--FontSize)", "var(--FontFamily)", "var(--LineHeight)"])
    );
    for declaration in [
        "font: INITIAL",
        "font-size: ReVeRt-LaYeR",
        "line-height: UnSeT",
    ] {
        assert!(
            cssom_parser::parse(&format!("x {{{declaration}}}")).contains(declaration),
            "{declaration}"
        );
    }
    assert!(!cssom_parser::parse("x {font: INITIALgarbage}").contains("font:"));
}

#[test]
fn important_border_computed_longhands_match_the_genuine_chrome_control() {
    use sha2::{Digest, Sha256};
    let source = include_str!("computed_border_priority_probe.js");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("computed_border_priority_chrome155.json")).unwrap();
    assert_eq!(
        fixture["probeSha256"],
        format!("{:x}", Sha256::digest(source.as_bytes()))
    );
    assert_eq!(
        fixture["htmlSha256"],
        format!(
            "{:x}",
            Sha256::digest(fixture["html"].as_str().unwrap().as_bytes())
        )
    );
    assert_eq!(fixture["browserVersion"], "Chrome/155.0.8059.85");
    assert_eq!(fixture["errors"], serde_json::json!([]));
    let actual = run(fixture["html"].as_str().unwrap(), source);
    let rows = actual.as_array().unwrap();
    let expected = fixture["result"].as_array().unwrap();
    assert_eq!(rows.len(), 9);
    assert_eq!(rows.len(), expected.len());
    // The canonical binding owns computed values; Surf's native layout test
    // separately asserts the same capture's clientLeft/clientTop columns.
    for (row, expected) in rows.iter().zip(expected) {
        assert_eq!(row[0], expected[0]);
        assert_eq!(
            &row.as_array().unwrap()[3..],
            &expected.as_array().unwrap()[3..]
        );
    }
}

#[test]
fn computed_importance_tracks_live_adoption_and_inline_priority_without_public_methods() {
    let actual = run(
        "<!doctype html><html><head><style>.target{padding-left:3px}</style></head><body><section style='width:256px'><div id=target class=target style='border:8px solid!important;color:blue;width:75%'></div></section></body></html>",
        r#"(()=>{
          const target=document.getElementById('target'),a=new CSSStyleSheet(),b=new CSSStyleSheet();
          a.replaceSync('.target{border-left-width:7px!important;color:#f00!important;width:50%!important;padding-left:5px}');
          b.replaceSync('.target{border-left-width:9px!important;color:#0f0!important}');
          document.adoptedStyleSheets=[a,b];
          const retained=getComputedStyle(target),rows=[],ordinaryFirst=retained.paddingLeft==='5px';
          function read(){rows.push([retained.borderLeftWidth,retained.borderTopWidth,retained.color,retained.width,retained.getPropertyPriority('border-left-width')]);}
          read();
          target.style.setProperty('border','8px solid');read();
          document.adoptedStyleSheets=[b,a];read();
          a.cssRules[0].style.setProperty('border-left-width','11px','important');read();
          const host=document.createElement('section');document.body.appendChild(host);const shadow=host.attachShadow({mode:'open'}),inside=document.createElement('div');inside.className='target';shadow.appendChild(inside);const shadowWidth=getComputedStyle(inside).borderLeftWidth;
          let reads=0;
          const priority=Object.getOwnPropertyDescriptor(CSSStyleDeclaration.prototype,'getPropertyPriority');
          Object.defineProperty(CSSStyleDeclaration.prototype,'getPropertyPriority',{...priority,value(){reads++;throw Error('public priority method reached');}});
          Object.defineProperty(target,'getRootNode',{configurable:true,value(){reads++;throw Error('public root method reached');}});
          Object.defineProperty(document,'adoptedStyleSheets',{configurable:true,get(){reads++;throw Error('public adoption getter reached');}});
          try{rows.push([retained.borderLeftWidth,retained.color,retained.width]);}
          finally{Object.defineProperty(CSSStyleDeclaration.prototype,'getPropertyPriority',priority);}
          return JSON.stringify({rows,reads,shadowWidth,ordinaryFirst});
        })()"#,
    );
    assert_eq!(
        actual,
        serde_json::json!({"rows":[
        ["8px","8px","rgb(0, 255, 0)","128px",""],
        ["9px","8px","rgb(0, 255, 0)","128px",""],
        ["7px","8px","rgb(255, 0, 0)","128px",""],
        ["11px","8px","rgb(255, 0, 0)","128px",""],
        ["11px","rgb(255, 0, 0)","128px"]
    ],"reads":0,"shadowWidth":"","ordinaryFirst":true})
    );
}

#[test]
fn computed_priority_uses_parsed_tokens_and_preserves_quoted_values() {
    let actual = run(
        "<!doctype html><html><head><style>#target{padding-left:1px ! IMPORTANT;padding-left:2px;border-top-width:3px !/**/important;border-top-width:4px;content:\"left;!important\"}</style></head><body><div id=target></div></body></html>",
        r#"JSON.stringify((()=>{const style=getComputedStyle(document.getElementById('target'));return [style.paddingLeft,style.borderTopWidth,style.content];})())"#,
    );
    assert_eq!(
        actual,
        serde_json::json!(["1px", "3px", "\"left;!important\""])
    );
}

#[test]
fn adopted_computed_sheet_copy_does_not_expose_private_collection_to_array_setters() {
    let actual = run(
        "<!doctype html><html><head></head><body><button id=target>x</button></body></html>",
        r#"(()=>{
          const target=document.getElementById('target'),sheet=new CSSStyleSheet();
          sheet.replaceSync('#target{border:4px solid!important}');document.adoptedStyleSheets=[sheet];
          const define=Object.defineProperty,previous=Object.getOwnPropertyDescriptor(Array.prototype,'0');
          let reads=0,value;
          define(Array.prototype,'0',{configurable:true,get(){return undefined;},set(item){
            if(item===sheet){reads++;throw Error('private sheet collection reached guest setter');}
            define(this,'0',{value:item,writable:true,enumerable:true,configurable:true});
          }});
          try{value=getComputedStyle(target).borderLeftWidth;}
          finally{if(previous)define(Array.prototype,'0',previous);else delete Array.prototype[0];}
          return JSON.stringify({value,reads,sameAdoption:document.adoptedStyleSheets[0]===sheet});
        })()"#,
    );
    assert_eq!(
        actual,
        serde_json::json!({"value":"4px","reads":0,"sameAdoption":true})
    );
}

// turbo-test#125: getComputedStyle reuses an element's computed style until something it reads
// changes; values are the ones a fresh computation reports. The document sheet scan every cascade
// computation starts with runs on the captured native `querySelectorAll` (turbo-dom#199), so a
// page-replaced `document.querySelectorAll` never observes it: warm, repeated and mutated reads
// all count zero page-visible scans.
#[test]
fn computed_style_reuses_the_cascade_until_the_document_changes() {
    // Cascade work is counted by the engine's own test-only hook (each fresh computed-style
    // calculation), not by wrapping a page-visible method the engine no longer calls.
    let value = run(
        "<!doctype html><head><style id=s>.box {color:rgb(1, 2, 3);width:200px} .box span {display:block}</style></head><body><div class=box id=a><p><span id=t style='width:50%'>x</span></p></div></body>",
        r#"(()=>{
          const runs=__cssTestCascadeRuns;
          const t=document.getElementById('t'),start=runs(),retained=getComputedStyle(t);
          const read=style=>[style.color,style.width,style.display,style.visibility,style.getPropertyValue('font-size')];
          const first=read(retained),warm=runs()-start;let mark=runs();const rows=[];
          for(let index=0;index<5;index++)rows.push(read(getComputedStyle(t)).concat(read(retained),[getComputedStyle(t.parentElement).color]));
          const repeated=runs()-mark;
          const distinct=getComputedStyle(t)!==getComputedStyle(t);
          t.setAttribute('data-x','1');mark=runs();const after=read(retained);const mutated=runs()-mark;
          mark=runs();read(retained);const settled=runs()-mark;
          return JSON.stringify({first,warm:warm>=4,repeated,row:rows[0],same:rows.every(row=>JSON.stringify(row)===JSON.stringify(rows[0])),distinct,after,mutated:mutated>0,settled});
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({
            "first": ["rgb(1, 2, 3)", "100px", "block", "visible", "16px"],
            "warm": true,
            "repeated": 0,
            "row": ["rgb(1, 2, 3)", "100px", "block", "visible", "16px", "rgb(1, 2, 3)", "100px", "block", "visible", "16px", "rgb(1, 2, 3)"],
            "same": true,
            "distinct": true,
            "after": ["rgb(1, 2, 3)", "100px", "block", "visible", "16px"],
            "mutated": true,
            "settled": 0
        })
    );
}

// turbo-test#125: a reused computed style is dropped by every input of the cascade — DOM, inline
// style, stylesheet text, CSSOM rules / declarations / selectors / disabled sheets, adopted
// sheets, form and top-layer state, animations and disconnection. Each step reads a fresh
// declaration and one retained from the start.
#[test]
fn computed_style_reuse_is_invalidated_by_every_cascade_input() {
    let value = run(
        "<!doctype html><head><style id=s>.box {color:rgb(1, 2, 3)} .hot {color:rgb(9, 9, 9)} #b {color:rgb(0, 0, 255)} #c:checked + #l {color:rgb(0, 128, 0)} #v:invalid {color:rgb(255, 0, 0)} #v:valid + #vl {color:rgb(0, 128, 0)} #q:popover-open {color:rgb(0, 0, 255)}</style></head><body><div class=box id=a><span id=t>x</span></div><div id=b></div><input type=checkbox id=c><label id=l for=c>l</label><input id=v required><label id=vl for=v>v</label><div popover id=p>p</div><div popover=manual id=q><span id=qc>q</span></div><dialog id=d>d</dialog></body>",
        r#"(()=>{
          const $=id=>document.getElementById(id),t=$('t'),a=$('a'),b=$('b'),s=$('s'),retained=getComputedStyle(t),rows=[];
          const step=(name,el,prop)=>{const fresh=getComputedStyle(el).getPropertyValue(prop);rows.push([name,fresh,el===t?retained.getPropertyValue(prop):fresh]);};
          step('warm',t,'color');step('warm-again',t,'color');
          t.className='hot';step('class',t,'color');t.className='';step('class-removed',t,'color');
          b.appendChild(t);step('moved',t,'color');a.appendChild(t);step('moved-back',t,'color');
          t.style.color='rgb(4, 4, 4)';step('inline',t,'color');t.removeAttribute('style');step('inline-removed',t,'color');
          s.textContent+=' #t {opacity:0.5}';step('style-text',t,'opacity');
          const sheet=s.sheet;sheet.insertRule('#t {opacity:0.25}',sheet.cssRules.length);step('insert-rule',t,'opacity');
          const rule=sheet.cssRules[sheet.cssRules.length-1];rule.style.opacity='0.125';step('rule-style',t,'opacity');
          rule.selectorText='#none';step('selector-text',t,'opacity');
          sheet.deleteRule(0);step('delete-rule',t,'color');
          sheet.disabled=true;step('disabled',t,'opacity');sheet.disabled=false;step('enabled',t,'opacity');
          const adopted=new CSSStyleSheet();adopted.replaceSync('#t {opacity:0.75}');
          document.adoptedStyleSheets=[adopted];step('adopted',t,'opacity');
          adopted.replaceSync('#t {opacity:0.625}');step('adopted-replaced',t,'opacity');
          document.adoptedStyleSheets.pop();step('adopted-popped',t,'opacity');
          document.adoptedStyleSheets.push(adopted);step('adopted-pushed',t,'opacity');
          document.adoptedStyleSheets.length=0;step('adopted-cleared',t,'opacity');
          step('unchecked',$('l'),'color');$('c').checked=true;step('checked',$('l'),'color');$('c').checked=false;step('checked-again',$('l'),'color');
          step('invalid',$('v'),'color');step('invalid-label',$('vl'),'color');$('v').value='x';step('valid',$('v'),'color');step('valid-label',$('vl'),'color');$('v').value='';step('invalid-again',$('v'),'color');step('invalid-label-again',$('vl'),'color');
          step('popover-child',$('qc'),'color');$('q').showPopover();step('popover-child-open',$('qc'),'color');$('q').hidePopover();step('popover-child-closed',$('qc'),'color');
          step('popover',$('p'),'display');$('p').showPopover();step('popover-open',$('p'),'display');$('p').hidePopover();step('popover-closed',$('p'),'display');
          step('dialog',$('d'),'position');$('d').showModal();step('dialog-modal',$('d'),'position');$('d').close();step('dialog-closed',$('d'),'position');
          const animation=t.animate([{opacity:0.1},{opacity:0.1}],{duration:100000,delay:50000});step('animation-delayed',t,'opacity');animation.currentTime=60000;step('animated',t,'opacity');animation.currentTime=0;step('animation-rewound',t,'opacity');animation.currentTime=60000;animation.cancel();step('animation-cancelled',t,'opacity');
          let clock=1000,frames=[];__cssTestNs.installAnimationHost({now:()=>clock,frame(handler){frames.push(handler);}});
          const frame=()=>{const handlers=frames.splice(0);for(const handler of handlers){handler.update();handler.dispatch();handler.commit();}};
          const timed=t.animate([{opacity:0.2},{opacity:0.2}],{duration:100000,delay:30});frame();step('timeline-delayed',t,'opacity');clock+=60;step('timeline-animated',t,'opacity');timed.cancel();
          t.remove();step('removed',t,'color');
          return JSON.stringify(rows);
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["warm", "rgb(1, 2, 3)", "rgb(1, 2, 3)"],
            ["warm-again", "rgb(1, 2, 3)", "rgb(1, 2, 3)"],
            ["class", "rgb(9, 9, 9)", "rgb(9, 9, 9)"],
            ["class-removed", "rgb(1, 2, 3)", "rgb(1, 2, 3)"],
            ["moved", "rgb(0, 0, 255)", "rgb(0, 0, 255)"],
            ["moved-back", "rgb(1, 2, 3)", "rgb(1, 2, 3)"],
            ["inline", "rgb(4, 4, 4)", "rgb(4, 4, 4)"],
            ["inline-removed", "rgb(1, 2, 3)", "rgb(1, 2, 3)"],
            ["style-text", "0.5", "0.5"],
            ["insert-rule", "0.25", "0.25"],
            ["rule-style", "0.125", "0.125"],
            ["selector-text", "0.5", "0.5"],
            ["delete-rule", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["disabled", "1", "1"],
            ["enabled", "0.5", "0.5"],
            ["adopted", "0.75", "0.75"],
            ["adopted-replaced", "0.625", "0.625"],
            ["adopted-popped", "0.5", "0.5"],
            ["adopted-pushed", "0.625", "0.625"],
            ["adopted-cleared", "0.5", "0.5"],
            ["unchecked", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["checked", "rgb(0, 128, 0)", "rgb(0, 128, 0)"],
            ["checked-again", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["invalid", "rgb(255, 0, 0)", "rgb(255, 0, 0)"],
            ["invalid-label", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["valid", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["valid-label", "rgb(0, 128, 0)", "rgb(0, 128, 0)"],
            ["invalid-again", "rgb(255, 0, 0)", "rgb(255, 0, 0)"],
            ["invalid-label-again", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["popover-child", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["popover-child-open", "rgb(0, 0, 255)", "rgb(0, 0, 255)"],
            ["popover-child-closed", "rgb(0, 0, 0)", "rgb(0, 0, 0)"],
            ["popover", "none", "none"],
            ["popover-open", "block", "block"],
            ["popover-closed", "none", "none"],
            ["dialog", "absolute", "absolute"],
            ["dialog-modal", "fixed", "fixed"],
            ["dialog-closed", "absolute", "absolute"],
            ["animation-delayed", "0.5", "0.5"],
            ["animated", "0.1", "0.1"],
            ["animation-rewound", "0.5", "0.5"],
            ["animation-cancelled", "0.5", "0.5"],
            ["timeline-delayed", "0.5", "0.5"],
            ["timeline-animated", "0.2", "0.2"],
            ["removed", "", ""]
        ])
    );
}

// turbo-test#125: an `@import` whose media query decides whether its rules apply is re-evaluated
// when the media result changes, even though nothing in the document did.
#[test]
fn computed_style_reuse_follows_import_media_changes() {
    let value = run(
        "<!doctype html><head><style id=s>@import url('child.css') screen; #t {height:17px}</style></head><body><div id=t></div></body>",
        r#"(()=>{
          let screen=true;globalThis.matchMedia=query=>({matches:screen,media:query});
          const sheet=document.getElementById('s').sheet,child=new CSSStyleSheet();
          __cssTestHost.attachImport(sheet.cssRules[0],child,'http://localhost/child.css','#t {width:33px;height:11px}',true);
          const t=document.getElementById('t'),retained=getComputedStyle(t),rows=[];
          const read=()=>rows.push([getComputedStyle(t).width,retained.width,retained.height]);
          read();read();screen=false;read();screen=true;read();
          return JSON.stringify(rows);
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["33px", "33px", "17px"],
            ["33px", "33px", "17px"],
            ["", "", "17px"],
            ["33px", "33px", "17px"]
        ])
    );
}

// A reused computed style that read live form / popover / dialog state is keyed on the binding's
// state epoch: every state change that can flip :placeholder-shown, :valid / :invalid,
// :indeterminate, :checked (options), :out-of-range, :user-invalid, a form-associated custom
// element's validity, :popover-open or :modal drops it, also when
// the state is read through :has() or a sibling combinator. Each target is read (and so cached)
// before every flip; a retained declaration re-reads live.
#[test]
fn computed_style_reuse_follows_form_popover_and_dialog_state() {
    let value = run(
        "<!doctype html><head><style>#v:placeholder-shown {color:rgb(1, 0, 0)} #f:has(:invalid) #fl {color:rgb(2, 0, 0)} #v:invalid + #vl {color:rgb(3, 0, 0)} #cv:invalid {color:rgb(4, 0, 0)} #ind:indeterminate + #indl {color:rgb(5, 0, 0)} #o2:checked {color:rgb(6, 0, 0)} #s:has(#o2:checked) + #sl {color:rgb(7, 0, 0)} #n:out-of-range {color:rgb(8, 0, 0)} #f2 :user-invalid {color:rgb(9, 0, 0)} #fc:invalid {color:rgb(10, 0, 0)} body:has(:popover-open) #pl {color:rgb(12, 0, 0)} body:has(:modal) #ml {color:rgb(13, 0, 0)}</style></head><body><form id=f><input id=v required placeholder=p><span id=vl>vl</span><span id=fl>fl</span></form><input id=cv><input type=checkbox id=ind><span id=indl>i</span><select id=s><option id=o1>a</option><option id=o2>b</option></select><span id=sl>s</span><input id=n type=number min=0 max=5><form id=f2><input id=u required></form><x-fc id=fc></x-fc><div popover=manual id=p>p</div><span id=pl>pl</span><dialog id=d>d</dialog><span id=ml>ml</span></body>",
        r#"(()=>{
          const $=id=>document.getElementById(id),retained={},rows=[];
          customElements.define('x-fc',class extends HTMLElement{static formAssociated=true;constructor(){super();this.i=this.attachInternals();}});
          const ids=['v','vl','fl','cv','indl','o2','sl','n','u','fc','pl','ml'];
          for(const id of ids){retained[id]=getComputedStyle($(id));retained[id].color;getComputedStyle($(id)).color;}
          const read=()=>{const row=[];for(const id of ids){const fresh=getComputedStyle($(id)).color,again=getComputedStyle($(id)).color,live=retained[id].color;row.push(fresh===again&&again===live?+fresh.slice(4).split(",")[0]:[fresh,again,live]);}return row;};
          rows.push(['initial'].concat(read()));
          const step=(name,flip)=>{read();flip();rows.push([name].concat(read()));};
          step('value',()=>{$('v').value='x';});step('value-cleared',()=>{$('v').value='';});
          step('custom-validity',()=>$('cv').setCustomValidity('bad'));step('custom-validity-cleared',()=>$('cv').setCustomValidity(''));
          step('indeterminate',()=>{$('ind').indeterminate=true;});step('indeterminate-cleared',()=>{$('ind').indeterminate=false;});
          step('option-selected',()=>{$('o2').selected=true;});step('selected-index',()=>{$('s').selectedIndex=0;});step('select-value',()=>{$('s').value='b';});
          step('out-of-range',()=>{$('n').value='9';});step('in-range',()=>{$('n').value='3';});
          step('user-validity',()=>$('f2').requestSubmit());step('user-validity-reset',()=>$('f2').reset());
          step('internals-invalid',()=>$('fc').i.setValidity({customError:true},'bad'));step('internals-valid',()=>$('fc').i.setValidity({}));
          step('popover-open',()=>$('p').showPopover());step('popover-closed',()=>$('p').hidePopover());
          step('modal',()=>$('d').showModal());step('modal-closed',()=>$('d').close());
          return JSON.stringify(rows);
        })()"#,
    );
    // Each row: the step, then the red channel of each target's color (rgb(N, 0, 0) is the Nth
    // rule; 0 = no state rule applies), in the order v vl fl cv indl o2 sl n u fc pl ml.
    assert_eq!(
        value,
        serde_json::json!([
            ["initial", 1, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ["value", 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ["value-cleared", 1, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ["custom-validity", 1, 3, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0],
            [
                "custom-validity-cleared",
                1,
                3,
                2,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0
            ],
            ["indeterminate", 1, 3, 2, 0, 5, 0, 0, 0, 0, 0, 0, 0],
            ["indeterminate-cleared", 1, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ["option-selected", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0],
            ["selected-index", 1, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ["select-value", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0],
            ["out-of-range", 1, 3, 2, 0, 0, 6, 7, 8, 0, 0, 0, 0],
            ["in-range", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0],
            ["user-validity", 1, 3, 2, 0, 0, 6, 7, 0, 9, 0, 0, 0],
            ["user-validity-reset", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0],
            ["internals-invalid", 1, 3, 2, 0, 0, 6, 7, 0, 0, 10, 0, 0],
            ["internals-valid", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0],
            ["popover-open", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 12, 0],
            ["popover-closed", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0],
            ["modal", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 13],
            ["modal-closed", 1, 3, 2, 0, 0, 6, 7, 0, 0, 0, 0, 0]
        ])
    );
}

// Perf gate: getComputedStyle reuses a computed style that read live form / popover / dialog
// state until that state (or anything else in the key) changes. Rows of controls styled by
// :disabled / :invalid / :has(:checked) rules re-read 5 times cost no fresh cascade after the
// first pass; a state flip recomputes, then reads settle again. Deterministic (a count of fresh
// computed-style calculations, not wall-clock).
#[test]
fn computed_style_state_rules_reuse_gate() {
    let value = run(
        "<!doctype html><head><style>button:disabled {opacity:0.5} input:invalid {color:rgb(1, 0, 0)} .row:has(:checked) {color:rgb(2, 0, 0)} .row:has(:popover-open) span {color:rgb(3, 0, 0)} dialog:modal + span {color:rgb(4, 0, 0)}</style></head><body></body>",
        r#"(()=>{
          const runs=__cssTestCascadeRuns;
          for(let i=0;i<20;i++){const row=document.createElement('div');row.className='row';row.innerHTML='<input required><input type=checkbox><button disabled>b</button><span>t</span><dialog>d</dialog><span>m</span>';document.body.appendChild(row);}
          const all=[...document.querySelectorAll('.row, .row *')];
          const pass=()=>{const out=[];for(const el of all){const style=getComputedStyle(el);out.push(style.color+'/'+style.opacity);}return out.join();};
          let mark=runs();const first=pass(),warm=runs()-mark;
          mark=runs();let same=true;for(let i=0;i<5;i++)same=pass()===first&&same;const repeated=runs()-mark;
          all[2].checked=true;mark=runs();const flipped=pass();const recomputed=runs()-mark;
          mark=runs();pass();pass();const settled=runs()-mark;
          return JSON.stringify({warm:warm>0,same,repeated,changed:flipped!==first,recomputed:recomputed>0,settled});
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"warm": true, "same": true, "repeated": 0, "changed": true, "recomputed": true, "settled": 0})
    );
}

// Perf gate (turbo-test#187 follow-up): the CSS animation / transition update runs once per
// style change of an element, not once per computed-style read. An element with a running
// animation recomputes its style on every read (the timeline moves), but its CSS animation /
// transition update reads only the cascade, so 50 reads of an unchanged element run no update
// pass; a style change runs one and starts its transition exactly as before.
#[test]
fn css_animation_update_runs_once_per_style_change_gate() {
    let value = run(
        "<!doctype html><head><style>#t {transition:opacity 100s linear, color 100s linear;opacity:1} #t.dim {opacity:0.5} @keyframes spin {from {width:10px} to {width:20px}} #a {animation:spin 100s linear}</style></head><body><div id=p><div id=t>t</div><div id=a>a</div></div></body>",
        r#"(()=>{
          const passes=__cssTestCssUpdates,t=document.getElementById('t'),a=document.getElementById('a');
          t.animate([{height:'1px'},{height:'2px'}],{duration:100000});
          const read=()=>[getComputedStyle(t).opacity,getComputedStyle(t).height!=='',getComputedStyle(a).width!==''];
          const first=read();let mark=passes();
          for(let i=0;i<50;i++)read();const repeated=passes()-mark;
          t.className='dim';mark=passes();read();const changed=passes()-mark;
          const kinds=t.getAnimations().map(animation=>animation.constructor.name+':'+(animation.transitionProperty||''));
          mark=passes();for(let i=0;i<50;i++)read();const settled=passes()-mark;
          const animations=a.getAnimations().map(animation=>animation.constructor.name+':'+animation.animationName);
          return JSON.stringify({first,repeated,changed:changed>0,kinds,settled,animations});
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"first": ["1", true, true], "repeated": 0, "changed": true, "kinds": ["CSSTransition:opacity", "Animation:"], "settled": 0, "animations": ["CSSAnimation:spin"]})
    );
}

// turbo-test#227: a transition update decides "rendered" (no display:none ancestor) from the
// ancestor styles the same computed-style read already resolves, so reading a transitioned
// element after a tree mutation costs no more cascade runs than reading an identical element
// without a transition. The ancestors run transitions of their own (volatile styles, never
// reused), as MUI form controls do.
#[test]
fn transition_update_resolves_no_extra_ancestor_styles_gate() {
    let value = run(
        "<!doctype html><head><style>.v {transition:opacity 100s linear;opacity:1} .v.dim {opacity:0.5} #t {display:block;height:1px;transition:height 100s linear}</style></head><body><div id=x></div><div class=v id=r1><div><div class=v><div><div><div><span id=t>t</span></div></div></div></div></div></div><div class=v id=r2><div><div class=v><div><div><div><span id=c>c</span></div></div></div></div></div></div></body>",
        r#"(()=>{
          const runs=__cssTestCascadeRuns,x=document.getElementById('x'),t=document.getElementById('t'),c=document.getElementById('c');
          for(const v of document.querySelectorAll('.v')){getComputedStyle(v).opacity;v.classList.add('dim');getComputedStyle(v).opacity;}
          for(const el of [t,c])getComputedStyle(el).height;
          const cost=el=>{let total=0;for(let i=0;i<20;i++){x.textContent=String(i);const mark=runs();getComputedStyle(el).height;total+=runs()-mark;}return total;};
          const transitioned=cost(t),control=cost(c);
          t.style.height='5px';getComputedStyle(t).height;
          const kinds=t.getAnimations().map(a=>a.constructor.name+':'+a.transitionProperty);
          return JSON.stringify({extra:transitioned-control,kinds,running:document.querySelectorAll('.v')[0].getAnimations().length});
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"extra": 0, "kinds": ["CSSTransition:height"], "running": 1})
    );
}

// turbo-test#227: an ancestor first resolved while a descendant's CSS update runs (the
// descendant inherits from it) still records its own before-change style, so a later change
// of the ancestor starts its transition, as Chrome's style recalc does.
#[test]
fn an_ancestor_resolved_inside_a_descendant_update_still_starts_its_transition() {
    let value = run(
        "<!doctype html><head><style>.v {transition:opacity 100s linear;opacity:1} .v.dim {opacity:0.5} #t {transition:color 100s linear}</style></head><body><div class=v id=v><span id=t>t</span></div></body>",
        r#"(()=>{
          const v=document.getElementById('v'),t=document.getElementById('t');
          getComputedStyle(t).color;
          v.classList.add('dim');
          const opacity=getComputedStyle(v).opacity;
          return JSON.stringify({opacity,kinds:v.getAnimations().map(a=>a.constructor.name+':'+a.transitionProperty)});
        })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"opacity": "1", "kinds": ["CSSTransition:opacity"]})
    );
}

// turbo-dom#113: the reuse key covers CSSOM edits made in another realm. A live declaration one
// realm returned for an element of another realm's document re-reads after that document's
// sheet is edited (a declaration write, an inserted rule), in both directions, while repeated
// reads of the unchanged element still scan no sheets. Chrome 155 control for the parent → child
// rows: turbo-surf harness/detection/fixtures/computed-style-owner-chrome155.json.
#[test]
fn computed_style_reuse_follows_cssom_edits_in_another_realm() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let parent = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, parent);
    install_with_html(
        scope,
        "<!doctype html><head><style id=s>.target {width:47px;color:red}</style></head><body><div class=target id=target></div></body>",
    );
    let child = v8::Context::new(scope, Default::default());
    child.set_security_token(parent.get_security_token(scope));
    {
        let scope = &mut v8::ContextScope::new(scope, child);
        install_independent_html(
            scope,
            "<!DOCTYPE html><html><head></head><body></body></html>",
        );
    }
    let child_global = child.global(scope);
    let parent_global = parent.global(scope);
    set_prop(scope, parent_global, "frameChild", child_global.into());
    let source = v8::String::new(
        scope,
        r#"JSON.stringify((()=>{
        const read=getComputedStyle,doc=frameChild.document,query=doc.querySelectorAll;let scans=0;
        const style=doc.createElement('style');style.textContent='.target {width:61px;color:blue}';doc.head.appendChild(style);
        const target=doc.createElement('div');target.className='target';doc.body.appendChild(target);
        const sheet=style.sheet,retained=read(target),rows=[];
        const row=name=>{const fresh=read(target);rows.push([name,retained.width,retained.color,retained.height,fresh.width,fresh.height]);};
        row('first');
        doc.querySelectorAll=function(selector){if(selector==='style,link')scans++;return Reflect.apply(query,this,arguments);};
        try {row('warm');row('warm-again');} finally {delete doc.querySelectorAll;}
        sheet.cssRules[0].style.width='67px';row('declaration');
        sheet.insertRule('.target {height:5px}',1);row('insert-rule');
        sheet.deleteRule(1);row('delete-rule');
        const own=document.getElementById('target'),childRead=frameChild.getComputedStyle,borrowed=childRead(own),reverse=[];
        const back=name=>reverse.push([name,borrowed.width,childRead(own).width]);
        back('first');back('warm');
        document.getElementById('s').sheet.cssRules[0].style.width='53px';back('declaration');
        return {rows,scans,reverse};
    })())"#,
    )
    .unwrap();
    let value = {
        v8::tc_scope!(let scope, scope);
        let value = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| {
                panic!(
                    "{:?}",
                    scope
                        .exception()
                        .map(|value| value.to_rust_string_lossy(scope))
                )
            });
        serde_json::from_str::<serde_json::Value>(&value.to_rust_string_lossy(scope)).unwrap()
    };
    assert_eq!(
        value,
        serde_json::json!({
            "rows": [
                ["first", "61px", "rgb(0, 0, 255)", "", "61px", ""],
                ["warm", "61px", "rgb(0, 0, 255)", "", "61px", ""],
                ["warm-again", "61px", "rgb(0, 0, 255)", "", "61px", ""],
                ["declaration", "67px", "rgb(0, 0, 255)", "", "67px", ""],
                ["insert-rule", "67px", "rgb(0, 0, 255)", "5px", "67px", "5px"],
                ["delete-rule", "67px", "rgb(0, 0, 255)", "", "67px", ""]
            ],
            "scans": 0,
            "reverse": [
                ["first", "47px", "47px"],
                ["warm", "47px", "47px"],
                ["declaration", "53px", "53px"]
            ]
        })
    );
    reset();
}

/// turbo-dom#199: the engine's uncurried `split` (ns.intrinsics.split) splits by a RegExp
/// without RegExp.prototype[@@split], with String.prototype.split's results: anchors, captures,
/// empty matches, flags, limits and the empty string.
#[test]
fn intrinsic_regexp_split_matches_string_split() {
    let value = run(
        "<!doctype html><body>",
        r#"(()=>{
          const split=__cssTestNs.intrinsics.split,out=[];
          const cases=[['a b  c',/\s+/],['a,b,(c,d)',/,(?![^(]*\))/],['abc',/(?:)/],['',/x/],['',/(?:)/],['^a^ab',/^a/],['aXa',/^a/m],['a1b2c',/(\d)/],['AbCb',/b/i],['x',/x/],['a  b',/ */],['a,b,c',/,/g],['a1b2c3',/\d/,2],['ab',/(?:)/,0],['a\nb',/^/m],['a😀b',/(?:)/u]];
          for(const [text,pattern,limit] of cases){const mine=JSON.stringify(split(text,pattern,limit)),native=JSON.stringify(text.split(pattern,limit));if(mine!==native)out.push(text+':'+pattern+':'+mine+'!='+native);}
          return JSON.stringify(out);
        })()"#,
    );
    assert_eq!(value, serde_json::json!([]));
}

/// Setting adoptedStyleSheets to a non-object throws the sequence conversion error before any
/// iteration, like Chrome 156 (measured headless on localhost): a string is never iterated.
#[test]
fn adopted_style_sheets_reject_non_objects_like_chrome156() {
    let value = run(
        "<!doctype html><body><div id=h></div>",
        r#"(()=>{ const out=[]; const h=document.getElementById('h').attachShadow({mode:'open'});
          for (const [t,n] of [[document,'d'],[h,'s']]) for (const v of ['x',5,null,'']) { try { t.adoptedStyleSheets=v; out.push(n+':ok:'+t.adoptedStyleSheets.length); } catch (e) { out.push(n+':'+e.constructor.name+':'+e.message); } }
          return JSON.stringify(out); })()"#,
    );
    let d = "d:TypeError:Failed to set the 'adoptedStyleSheets' property on 'Document': The provided value cannot be converted to a sequence.";
    let s = "s:TypeError:Failed to set the 'adoptedStyleSheets' property on 'ShadowRoot': The provided value cannot be converted to a sequence.";
    assert_eq!(value, serde_json::json!([d, d, d, d, s, s, s, s]));
}

/// A transitioned element's style read after a DOM change checks whether the element is
/// rendered (no display:none ancestor). That check resolves each ancestor once per read, not
/// once per ancestor per level: a read stays linear in the tree depth (MUI's
/// TextareaAutosize reads its textarea's style between mutations inside a deep dialog).
#[test]
fn rendered_check_of_a_transition_read_is_linear_in_depth() {
    let value = run(
        "<style>.t{transition:opacity 1s}</style><body></body>",
        r#"(()=>{
      const depth=40;let parent=document.body;
      for(let i=0;i<depth;i++){const d=document.createElement('div');parent.appendChild(d);parent=d;}
      const el=document.createElement('div');el.className='t';parent.appendChild(el);
      const other=document.createElement('span');document.body.appendChild(other);
      getComputedStyle(el).opacity;
      const counts=[];
      for(let i=0;i<3;i++){
        other.style.width=i+'px';
        const before=__cssTestStyleLookups();
        const opacity=getComputedStyle(el).opacity;
        counts.push(__cssTestStyleLookups()-before);
        if(opacity!=='1')throw Error('opacity '+opacity);
      }
      return JSON.stringify({depth,max:Math.max(...counts)});
    })()"#,
    );
    let depth = value["depth"].as_u64().unwrap();
    let max = value["max"].as_u64().unwrap();
    assert!(max > 0, "the hook counts lookups");
    assert!(
        max <= 8 * (depth + 2),
        "{max} style lookups for one read at depth {depth}"
    );
}

/// One computed-style read after a DOM change cascades the element and each ancestor afresh;
/// they all read the same document sheet list, so the list is read once per tree version and
/// CSSOM epoch, not once per element.
#[test]
fn a_cascade_reads_the_document_sheet_list_once_per_version() {
    let value = run(
        "<style>.t{color:red}</style><body></body>",
        r#"(()=>{
      let parent=document.body;
      for(let i=0;i<20;i++){const d=document.createElement('div');parent.appendChild(d);parent=d;}
      const el=document.createElement('div');el.className='t';parent.appendChild(el);
      const read=__cssTestHost.readSheets;let reads=0;
      __cssTestHost.readSheets=function(){reads++;return read.apply(this,arguments);};
      document.body.setAttribute('data-x','1');
      const color=getComputedStyle(el).color;
      const first=reads;
      const s=document.createElement('style');s.textContent='.t{color:blue}';document.head.appendChild(s);
      const after=getComputedStyle(el).color;
      return JSON.stringify({color,first,after});
    })()"#,
    );
    assert_eq!(value["color"], "rgb(255, 0, 0)");
    assert_eq!(value["after"], "rgb(0, 0, 255)", "a new sheet is seen");
    assert!(value["first"].as_u64().unwrap() <= 2, "{value}");
}

/// An inline style write to one element keeps every other element's computed style reused:
/// computed values read an element's own declarations and its parent's, and selectors see
/// inline styles only through attribute selectors naming `style`. MUI's TextareaAutosize
/// writes a hidden textarea's style between reads of its visible textarea's style, inside a
/// deep dialog: those reads must not cascade the textarea and all its ancestors again.
#[test]
fn an_inline_style_write_keeps_unrelated_computed_styles() {
    let value = run(
        "<style>.t{color:red} div{margin:0}</style><body></body>",
        r#"(()=>{
      let parent=document.body;
      for(let i=0;i<30;i++){const d=document.createElement('div');parent.appendChild(d);parent=d;}
      const target=document.createElement('textarea');target.className='t';parent.appendChild(target);
      const hidden=document.createElement('textarea');parent.appendChild(hidden);
      getComputedStyle(target).color;
      const runs=[];
      for(let i=0;i<5;i++){
        hidden.style.width=i+'px';
        const before=__cssTestCascadeRuns();
        const color=getComputedStyle(target).color;
        runs.push(__cssTestCascadeRuns()-before);
        if(color!=='rgb(255, 0, 0)')throw Error(color);
      }
      return JSON.stringify({max:Math.max(...runs),hidden:getComputedStyle(hidden).width});
    })()"#,
    );
    assert_eq!(
        value["hidden"], "4px",
        "the written element's own style follows"
    );
    assert_eq!(value["max"], 0, "{value}");
}

/// What an inline style write still changes, beyond its element: inherited values below it,
/// and every element an attribute selector naming `style` (directly, through a combinator or
/// inside `:has()`) can reach. Same values as a fresh cascade (Chrome's: inline styles are
/// the element's own declarations and the `style` attribute's value for selectors).
#[test]
fn an_inline_style_write_updates_inheritance_and_style_attribute_selectors() {
    let value = run(
        "<style>[style*=\"orange\"] + p{color:blue} section:has(> [style*=\"gold\"]){color:green}</style><body><div id=a><span id=s>x</span></div><b id=b></b><p id=p>p</p><section id=sec><i id=i></i></section></body>",
        r#"(()=>{
      const a=document.getElementById('a'),s=document.getElementById('s'),b=document.getElementById('b'),p=document.getElementById('p'),sec=document.getElementById('sec'),i=document.getElementById('i');
      const read=()=>[getComputedStyle(s).color,getComputedStyle(p).color,getComputedStyle(sec).color,getComputedStyle(a).color].join('|');
      const out=[read()];
      a.style.color='rgb(1, 2, 3)'; out.push(read());
      b.setAttribute('style','background:orange'); out.push(read());
      i.style.outlineColor='gold'; out.push(read());
      a.removeAttribute('style'); b.removeAttribute('style'); i.removeAttribute('style'); out.push(read());
      return JSON.stringify(out);
    })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!([
            "rgb(0, 0, 0)|rgb(0, 0, 0)|rgb(0, 0, 0)|rgb(0, 0, 0)",
            "rgb(1, 2, 3)|rgb(0, 0, 0)|rgb(0, 0, 0)|rgb(1, 2, 3)",
            "rgb(1, 2, 3)|rgb(0, 0, 255)|rgb(0, 0, 0)|rgb(1, 2, 3)",
            "rgb(1, 2, 3)|rgb(0, 0, 255)|rgb(0, 128, 0)|rgb(1, 2, 3)",
            "rgb(0, 0, 0)|rgb(0, 0, 0)|rgb(0, 0, 0)|rgb(0, 0, 0)"
        ])
    );
}

/// A page with a shadow tree keeps the plain version: a shadow tree's own sheets (`::slotted`,
/// `:host`) are outside the document sheets an embedder can check for `style` selectors.
#[test]
fn an_inline_style_write_in_a_shadow_page_still_updates_slotted_rules() {
    let value = run(
        "<body><div id=host><span id=light>x</span></div><em id=e></em></body>",
        r#"(()=>{
      const host=document.getElementById('host'),light=document.getElementById('light');
      const root=host.attachShadow({mode:'open'});
      root.innerHTML='<style>::slotted([style*="lime"]){color:rgb(9, 9, 9)}</style><slot></slot>';
      const before=getComputedStyle(light).color;
      light.style.backgroundColor='lime';
      return JSON.stringify([before,getComputedStyle(light).color]);
    })()"#,
    );
    assert_eq!(value, serde_json::json!(["rgb(0, 0, 0)", "rgb(9, 9, 9)"]));
}

/// An adopted sheet's attribute selector naming `style` still follows inline writes: the
/// sheet scan does not read adopted sheets, so their presence keeps the plain version. Same
/// values as a fresh cascade.
#[test]
fn style_attribute_selectors_in_adopted_sheets_still_follow_inline_writes() {
    let value = run(
        "<body><em id=e></em><q id=q>q</q></body>",
        r#"(()=>{
      const e=document.getElementById('e'),q=document.getElementById('q');
      const sheet=new CSSStyleSheet();sheet.replaceSync('[style*="teal"] + q { color: rgb(0, 128, 128) }');
      document.adoptedStyleSheets=[sheet];
      const out=[getComputedStyle(q).color];
      e.style.borderColor='teal';
      out.push(getComputedStyle(q).color);
      e.removeAttribute('style');
      out.push(getComputedStyle(q).color);
      return JSON.stringify(out);
    })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!(["rgb(0, 0, 0)", "rgb(0, 128, 128)", "rgb(0, 0, 0)"])
    );
}

/// The inline-style reuse cases (inheritance below a written element, `[style*=] + p`,
/// `:has(> [style*=])`, attribute removal, an unrelated element's writes deep in the tree, an
/// adopted sheet's and a shadow tree's `::slotted` `style` selectors), reads interleaved with
/// the writes, against the Chrome 156 control (`scripts/css-probe-chrome-control.mjs`).
#[test]
fn inline_style_reuse_matches_the_chrome_control() {
    use sha2::{Digest, Sha256};
    let source = include_str!("inline_style_reuse_probe.js");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("inline_style_reuse_chrome156.json")).unwrap();
    assert_eq!(
        fixture["probeSha256"],
        format!("{:x}", Sha256::digest(source.as_bytes()))
    );
    assert_eq!(fixture["browser"], "Chrome/156.0.8078.47");
    let actual = run(
        "<!doctype html><html><head></head><body></body></html>",
        source,
    );
    assert_eq!(actual, fixture["result"]);
}

/// A computation that read a rule under a conditional @media is reused while every media
/// condition it evaluated still gives the same `matchMedia` answer (a page or test can swap
/// `matchMedia`, so the answers are checked again, not assumed): MUI puts @media rules on most
/// elements, and never reusing those styles made every read a fresh cascade.
#[test]
fn media_rule_styles_are_reused_until_a_media_answer_changes() {
    let value = run(
        "<style>p{color:rgb(1, 1, 1)} @media (min-width: 600px) { p { color: rgb(2, 2, 2) } }</style><body><p id=p>p</p></body>",
        r#"(()=>{
      let wide=true;globalThis.matchMedia=query=>({matches:query.includes('600px')?wide:false,media:query});
      const p=document.getElementById('p');
      const first=getComputedStyle(p).color;
      const before=__cssTestCascadeRuns();
      const again=getComputedStyle(p).color;
      const reused=__cssTestCascadeRuns()-before;
      wide=false;
      const narrow=getComputedStyle(p).color;
      return JSON.stringify({first,again,reused,narrow});
    })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"first":"rgb(2, 2, 2)","again":"rgb(2, 2, 2)","reused":0,"narrow":"rgb(1, 1, 1)"})
    );
}

/// The document's rule entries (style rules under their grouping rules, layers and @media
/// answers) are built once per sheet list, CSSOM epoch and media answers, not once per element
/// whose style is computed: a read after a DOM change cascades the element and each ancestor.
#[test]
fn rule_entries_are_built_once_per_sheet_list() {
    let value = run(
        "<style>@layer base { div { color: rgb(1, 1, 1) } } @media (min-width: 1px) { .t { color: rgb(2, 2, 2) } } @supports (color: red) { .u { margin: 1px } }</style><body></body>",
        r#"(()=>{
      globalThis.matchMedia=query=>({matches:true,media:query});
      let parent=document.body;
      for(let i=0;i<20;i++){const d=document.createElement('div');parent.appendChild(d);parent=d;}
      const t=document.createElement('p');t.className='t u';parent.appendChild(t);
      const first=getComputedStyle(t).color;
      document.body.setAttribute('data-x','1');
      const before=__cssTestEntryBuilds();
      const color=getComputedStyle(t).color, margin=getComputedStyle(t).marginTop, inherited=getComputedStyle(parent).color;
      const builds=__cssTestEntryBuilds()-before;
      const sheet=document.querySelector('style').sheet;sheet.insertRule('.t { color: rgb(3, 3, 3) }', sheet.cssRules.length);
      return JSON.stringify({first,color,margin,inherited,builds,after:getComputedStyle(t).color});
    })()"#,
    );
    assert_eq!(value["first"], "rgb(2, 2, 2)");
    assert_eq!(value["color"], "rgb(2, 2, 2)");
    assert_eq!(value["margin"], "1px");
    assert_eq!(value["inherited"], "rgb(1, 1, 1)");
    assert_eq!(value["after"], "rgb(3, 3, 3)", "a CSSOM change rebuilds");
    assert!(value["builds"].as_u64().unwrap() <= 1, "{value}");
}
