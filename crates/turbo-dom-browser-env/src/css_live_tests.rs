use super::*;

fn run(html: &str, code: &str) -> serde_json::Value {
    run_with_font_defaults(html, code, None)
}

fn run_with_font_defaults(html: &str, code: &str, fonts: Option<(&str, f64)>) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
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
    let isolate = &mut v8::Isolate::new(Default::default());
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
