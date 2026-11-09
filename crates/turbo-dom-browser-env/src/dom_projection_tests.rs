use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;

#[test]
fn replaced_document_preserves_traversal_and_equality_on_all_native_node_kinds() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    for _ in 0..3 {
        install_independent_html(scope, "<!doctype html><body><div></div></body>");
        let code = v8::String::new(scope, r#"(()=>{
          const host=document.querySelector('div'),root=host.attachShadow({mode:'open'});
          root.innerHTML='<b>shadow</b>tail';
          const nodes=[root,document.createDocumentFragment(),document.createElement('section')];
          for(const node of nodes){
            if(node!==root)node.append(document.createElement('b'),document.createTextNode('tail'));
            if(node.lastChild!==node.childNodes.item(1)||node.lastChild.nodeName!=='#text'||node.lastChild.previousSibling!==node.firstChild)throw Error('native traversal');
            if(!node.isSameNode(node)||!node.isEqualNode(node)||node!==root&&!node.isEqualNode(node.cloneNode(true)))throw Error('native equality');
          }
          for(const name of ['lastChild','previousSibling','isSameNode','isEqualNode']){
            const descriptor=Object.getOwnPropertyDescriptor(Node.prototype,name);
            if(!descriptor||!descriptor.enumerable||!descriptor.configurable)throw Error('interface ownership '+name);
            const method=descriptor.get||descriptor.value;
            if(!Function.prototype.toString.call(method).includes('[native code]'))throw Error('native delegate '+name);
            let invalid=false;try{method.call({});}catch(error){invalid=error.name==='TypeError';}
            if(!invalid)throw Error('native receiver brand '+name);
          }
          return 'ok';
        })()"#).unwrap();
        v8::tc_scope!(let catch, scope);
        let result = v8::Script::compile(catch, code, None)
            .and_then(|script| script.run(catch))
            .unwrap_or_else(|| panic!("{}", thrown(catch.exception(), catch)));
        assert_eq!(result.to_rust_string_lossy(catch), "ok");
    }
    reset();
}

#[test]
fn first_native_wrapping_does_not_call_page_modified_projection_intrinsics() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body></body>");
    let code = v8::String::new(scope, r#"(()=>{
      const originals=[Function.prototype.call,Object.setPrototypeOf,Object.prototype.hasOwnProperty,
        WeakSet.prototype.has,WeakSet.prototype.add,Array.prototype.forEach,String.prototype.toUpperCase,String.prototype.slice];
      let hooks=0;const poison=()=>{hooks++;throw Error('page projection intrinsic '+new Error().stack);};
      let nodes;
      try {
        Function.prototype.call=poison;Object.setPrototypeOf=poison;Object.prototype.hasOwnProperty=poison;
        WeakSet.prototype.has=poison;WeakSet.prototype.add=poison;Array.prototype.forEach=poison;
        String.prototype.toUpperCase=poison;String.prototype.slice=poison;
        nodes=[document.createTextNode('text'),document.createComment('comment'),
          document.createAttribute('name'),document.createElement('section'),
          document.createElementNS('http://www.w3.org/2000/svg','svg'),document.createDocumentFragment()];
      } finally {
        Function.prototype.call=originals[0];Object.setPrototypeOf=originals[1];Object.prototype.hasOwnProperty=originals[2];
        WeakSet.prototype.has=originals[3];WeakSet.prototype.add=originals[4];Array.prototype.forEach=originals[5];
        String.prototype.toUpperCase=originals[6];String.prototype.slice=originals[7];
      }
      if(hooks!==0)throw Error('page hooks ran');
      const types=[Text,Comment,Attr,HTMLElement,SVGSVGElement,DocumentFragment];
      for(let index=0;index<nodes.length;index++)if(!(nodes[index] instanceof types[index]))throw Error('incorrect projection '+index);
      return 'ok';
    })()"#).unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

#[test]
fn parser_created_controls_use_private_projectors_despite_page_constructor_getters() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<body><input id='latent-input' value='native'></body>",
    );
    let code = concat!(
        "return ",
        r#"(()=>{
      const ns=__tsNs,define=Object.defineProperty;
      const inputPrototype=HTMLInputElement.prototype,textareaPrototype=HTMLTextAreaElement.prototype;
      const inputDescriptor=Object.getOwnPropertyDescriptor(globalThis,'HTMLInputElement');
      const textareaDescriptor=Object.getOwnPropertyDescriptor(globalThis,'HTMLTextAreaElement');
      const projectionDescriptor=Object.getOwnPropertyDescriptor(ns,'projectNativeNode');
      let hooks=0,input,textarea,inert,custom;
      const poison=()=>{hooks++;throw Error('page node projection getter');};
      try {
        define(globalThis,'HTMLInputElement',{get:poison,configurable:true});
        define(globalThis,'HTMLTextAreaElement',{get:poison,configurable:true});
        define(ns,'projectNativeNode',{get:poison,configurable:true});
        define(Object.prototype,'X-CUSTOM',{value:'HTMLInputElement',configurable:true});
        input=document.getElementById('latent-input');
        textarea=document.createElement('textarea');
        inert=document.implementation.createHTMLDocument('private');
        custom=inert.createElement('x-custom');
        inert.body.innerHTML='<input id="inert-input" value="inert">';
        if(inert.getElementById('inert-input').value!=='inert')throw Error('independent control');
      } finally {
        define(globalThis,'HTMLInputElement',inputDescriptor);
        define(globalThis,'HTMLTextAreaElement',textareaDescriptor);
        define(ns,'projectNativeNode',projectionDescriptor);
        delete Object.prototype['X-CUSTOM'];
      }
      if(hooks!==0||Object.getPrototypeOf(input)!==inputPrototype||input.value!=='native'||Object.getPrototypeOf(textarea)!==textareaPrototype)throw Error('projection intercepted');
      if(Object.getPrototypeOf(custom)!==HTMLElement.prototype||custom.ownerDocument!==inert)throw Error('inherited interface name');
      return 'ok';
    })()"#
    );
    v8::tc_scope!(let catch, scope);
    let result = crate::browser_env::run_engine_script(catch, code)
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

#[test]
fn interrupted_native_wrapping_leaves_no_unprojected_cached_object() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body></body>");
    let id = context.get_slot::<DomContextId>().unwrap().0;
    let _dom = DomScope::enter(id);
    let handle = with_tree_mut(|tree| tree.create_element("input")).unwrap();
    v8::tc_scope!(let scope, scope);
    scope.terminate_execution();
    let _discarded = wrap(scope, handle);
    // Weak-handle localization can materialize the interrupt into TryCatch
    // before JavaScript runs; the isolate pending flag is then already cleared.
    assert!(scope.is_execution_terminating() || scope.has_terminated());
    assert!(DOM.with(|dom| !dom.borrow().as_ref().unwrap().cache.contains_key(&handle)));
    scope.cancel_terminate_execution();
    scope.reset();
    let node = wrap(scope, handle);
    let key = v8::String::new(scope, "value").unwrap();
    assert_eq!(
        node.get(scope, key.into())
            .unwrap()
            .to_rust_string_lossy(scope),
        ""
    );
    reset();
}

#[test]
fn html_namespace_local_names_and_foreign_control_brands_match_chrome() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><body></body>");
    let probe = include_str!("element_interface_probe.js");
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)));
    let actual: serde_json::Value =
        serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("element_interface_chrome155.json")).unwrap();
    let actual = actual.as_array().unwrap();
    let expected = expected["result"].as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        if expected["namespace"] == "http://www.w3.org/2000/svg" {
            // SVG-specific interfaces are a separate implementation backlog.
            for field in [
                "kind",
                "namespace",
                "name",
                "localName",
                "prefix",
                "tagName",
                "html",
                "input",
                "unknown",
                "scriptAsync",
                "inputType",
            ] {
                assert_eq!(
                    actual[field], expected[field],
                    "foreign control brand row {index}: {field}"
                );
            }
        } else {
            assert_eq!(
                actual, expected,
                "HTML/null-namespace interface row {index}"
            );
        }
    }
    reset();
}

/// miaskiewicz/turbo-dom#396: the canvas fillStyle / strokeStyle paint-set runs after page
/// JS and must not look up the page's `WeakSet.prototype.add` / `.has` (turbo-surf#87 /
/// turbo-dom#172 intrinsic-capture contract). A hostile page replaces every configurable
/// prototype method with a thrower; the bootstrap's own paint-brand bookkeeping has to use
/// the natives it captured at install time.
#[test]
fn canvas_paint_bookkeeping_does_not_call_page_replaced_weakset_members() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><body></body>");
    let code = v8::String::new(
        scope,
        r#"(()=>{
      const canvas=document.createElement('canvas');document.body.appendChild(canvas);
      const ctx=canvas.getContext('2d');
      const originals=[WeakSet.prototype.has,WeakSet.prototype.add];
      let hooks=0;const poison=function(){hooks++;throw Error('patched WeakSet');};
      let gradient,pattern;
      try {
        WeakSet.prototype.has=poison;WeakSet.prototype.add=poison;
        gradient=ctx.createLinearGradient(0,0,1,1);
        pattern=ctx.createPattern(canvas,'repeat');
        ctx.fillStyle=gradient;
        ctx.strokeStyle=pattern;
      } finally {
        WeakSet.prototype.has=originals[0];WeakSet.prototype.add=originals[1];
      }
      if(hooks!==0)throw Error('page hooks ran '+hooks);
      if(ctx.fillStyle!==gradient||ctx.strokeStyle!==pattern)throw Error('paint not retained');
      return 'ok';
    })()"#,
    )
    .unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

/// miaskiewicz/turbo-dom#396 (wider): every WeakSet the bootstrap keeps (the canvas
/// paint-brand, the select-element emptySelects, the EventTarget-instance brand, the
/// element-projection cache, the XMLSerializer-instance brand, the TextTrackCueList brand)
/// is exercised AFTER the page has replaced `WeakSet.prototype.{add,has,delete}` with
/// throwers. Every one must dispatch through an intrinsic captured at install time
/// (`$c.{add,has,delete}` or a module-init `WeakSet.prototype.*` binding) so no live
/// prototype lookup reaches the page's poison.
#[test]
fn every_weakset_bookkeeping_site_uses_captured_intrinsics() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<!doctype html><body><select id='s'><option id='o1'>a</option><option id='o2'>b</option></select></body>",
    );
    let code = v8::String::new(
        scope,
        r#"(()=>{
      // Elements and instances are prepared BEFORE the poison lands: the test proves
      // these engine paths still work after the page replaces WeakSet.prototype.*.
      const sel=document.getElementById('s'),opt2=document.getElementById('o2');
      const canvas=document.createElement('canvas');document.body.appendChild(canvas);
      const ctx=canvas.getContext('2d');
      const trackHost=document.createElement('video');document.body.appendChild(trackHost);
      const track=trackHost.addTextTrack('subtitles','en');
      const serializer=new XMLSerializer();
      const originals={__proto__:null,
        add:WeakSet.prototype.add,has:WeakSet.prototype.has,delete:WeakSet.prototype.delete};
      let hooks=0;const trip=function(){hooks++;throw new Error('patched WeakSet')};
      const results=[];const attempt=(name,f)=>{try{results.push([name,'ok',f()])}catch(e){results.push([name,'threw',e.message])}};
      try {
        WeakSet.prototype.add=trip;WeakSet.prototype.has=trip;WeakSet.prototype.delete=trip;
        // Canvas paint brand (__cvPaints): fillStyle gradient/pattern.
        attempt('canvas-paint',()=>{const g=ctx.createLinearGradient(0,0,1,1);ctx.fillStyle=g;return ctx.fillStyle===g;});
        // Select / option selected state (emptySelects WeakSet).
        attempt('select-value-reset',()=>{sel.value='';return sel.selectedIndex===-1||sel.selectedIndex===0;});
        attempt('select-value-set',()=>{sel.value='b';return sel.value==='b';});
        attempt('select-selectedIndex-set',()=>{sel.selectedIndex=0;return sel.selectedIndex===0;});
        attempt('option-selected-set',()=>{opt2.selected=true;return opt2.selected===true;});
        // Form reset (resetSelectDelete captures WeakSet.prototype.delete).
        attempt('form-reset-control',()=>{const f=document.createElement('form');f.appendChild(sel);f.reset();f.removeChild(sel);document.body.appendChild(sel);return true;});
        // EventTarget-instance brand (eventTargets WeakSet): construct and addEventListener.
        attempt('event-target-ctor',()=>{const et=new EventTarget();et.addEventListener('x',()=>{});return et instanceof EventTarget;});
        // Native node projection (projected WeakSet) — first wrap of a fresh native node.
        attempt('project-native-node',()=>{const d=document.createElement('div');document.body.appendChild(d);return d instanceof HTMLDivElement;});
        // XMLSerializer-instance brand (xml_documents.js instances WeakSet): captured at module init.
        attempt('xml-serializer',()=>serializer.serializeToString(document.createElement('div')).length>0);
        // HTMLTrackElement's TextTrackCueList brand (html_track.js cueLists WeakSet).
        attempt('track-cues',()=>track.cues===null||track.cues.length===0);
      } finally {
        WeakSet.prototype.add=originals.add;WeakSet.prototype.has=originals.has;WeakSet.prototype.delete=originals.delete;
      }
      const failed=results.filter(r=>r[1]!=='ok').map(r=>r[0]+':'+r[2]);
      if(hooks!==0)throw new Error('live WeakSet hook ran '+hooks+' times; '+JSON.stringify(results));
      if(failed.length)throw new Error('site failed: '+failed.join(' | '));
      return 'ok';
    })()"#,
    )
    .unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

/// Review R3 M6: the WeakSet audit above exercises `browser_env.js`,
/// `select_element.js`, `xml_documents.js` and `html_track.js`. The
/// sibling bootstrap files that keep per-realm registries through
/// captured `Set.prototype.*` / `Map.prototype.*` / `WeakSet.prototype.*`
/// (`event_listeners.js`, `trusted_types.js`, `worker_scope.js`) are not
/// explicitly probed by that test. Extend the audit: poison every
/// bookkeeping prototype method the bootstrap could reach through live
/// `globalThis.{Set,Map,WeakSet,WeakMap}`, then exercise the three
/// sibling hot paths. If any of them leaked a live prototype lookup, the
/// thrower would fire and the test would fail.
#[test]
fn sibling_bootstrap_files_use_captured_intrinsics_under_prototype_poison() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><body></body>");
    let code = v8::String::new(
        scope,
        r#"(()=>{
      const originals={__proto__:null,
        setAdd:Set.prototype.add,setDel:Set.prototype.delete,setHas:Set.prototype.has,
        mapGet:Map.prototype.get,mapSet:Map.prototype.set,mapDel:Map.prototype.delete,
        wsAdd:WeakSet.prototype.add,wsHas:WeakSet.prototype.has,wsDel:WeakSet.prototype.delete,
        wmGet:WeakMap.prototype.get,wmSet:WeakMap.prototype.set,wmHas:WeakMap.prototype.has};
      const results=[];let hooks=0;
      const trip=function(){hooks++;throw new Error('live prototype read')};
      const attempt=(name,f)=>{try{results.push([name,'ok',f()])}catch(e){results.push([name,'threw',e.message])}};
      try {
        for(const key of ['add','delete','has']) Set.prototype[key]=trip;
        for(const key of ['get','set','delete']) Map.prototype[key]=trip;
        for(const key of ['add','has','delete']) WeakSet.prototype[key]=trip;
        for(const key of ['get','set','has']) WeakMap.prototype[key]=trip;
        attempt('window-add-dispatch',()=>{
          let hit=0; const handler=()=>{hit++};
          window.addEventListener('zoom',handler);
          window.dispatchEvent(new Event('zoom'));
          window.removeEventListener('zoom',handler);
          return hit===1;
        });
        attempt('event-target-chain',()=>{
          let hit=0; const t=new EventTarget();
          t.addEventListener('ping',()=>{hit++});
          t.dispatchEvent(new Event('ping'));
          return hit===1;
        });
        attempt('trusted-types-factory',()=>{
          return typeof trustedTypes==='object';
        });
        attempt('trusted-types-policy',()=>{
          const p = trustedTypes.createPolicy('r3m6', { createHTML: s => s });
          return p.createHTML('ok') instanceof TrustedHTML;
        });
      } finally {
        Set.prototype.add=originals.setAdd;Set.prototype.delete=originals.setDel;Set.prototype.has=originals.setHas;
        Map.prototype.get=originals.mapGet;Map.prototype.set=originals.mapSet;Map.prototype.delete=originals.mapDel;
        WeakSet.prototype.add=originals.wsAdd;WeakSet.prototype.has=originals.wsHas;WeakSet.prototype.delete=originals.wsDel;
        WeakMap.prototype.get=originals.wmGet;WeakMap.prototype.set=originals.wmSet;WeakMap.prototype.has=originals.wmHas;
      }
      const failed=results.filter(r=>r[1]!=='ok').map(r=>r[0]+':'+r[2]);
      if(hooks!==0)throw new Error('live prototype hook ran '+hooks+' times; '+JSON.stringify(results));
      if(failed.length)throw new Error('site failed: '+failed.join(' | '));
      return 'ok';
    })()"#,
    )
    .unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

/// turbo-dom#449: page JS that patches `Promise[Symbol.species]` (and every other
/// well-known species getter) BEFORE an iframe is created must not poison the
/// `iframe.contentWindow` build path — the engine's `@@species` reads on this path
/// must dispatch through bootstrap-captured references.
#[test]
fn every_species_getter_used_by_iframe_install_is_captured_at_bootstrap() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><body></body>");
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__capturedSpecies = {
            PromiseSpecies: __tsNs.PromiseSpecies,
            ArraySpecies: __tsNs.ArraySpecies,
            MapSpecies: __tsNs.MapSpecies,
            SetSpecies: __tsNs.SetSpecies,
            RegExpSpecies: __tsNs.RegExpSpecies,
            ArrayBufferSpecies: __tsNs.ArrayBufferSpecies,
         };",
    )
    .unwrap();
    let code = v8::String::new(
        scope,
        r#"(()=>{
      const gopd=Object.getOwnPropertyDescriptor,def=Object.defineProperty;
      const f=document.createElement('iframe');
      const saved=[];let hooks=0;const trips=[];
      const thrower=(label)=>function(){hooks++;trips.push(label);throw new Error('patched '+label)};
      const ctors=[['Array',Array],['Promise',Promise],['Map',Map],['Set',Set],
                   ['RegExp',RegExp],['ArrayBuffer',ArrayBuffer]];
      for(const [name,C] of ctors){
        const desc=gopd(C,Symbol.species);
        if(!desc||!desc.configurable)continue;
        saved.push([C,Symbol.species,desc]);
        def(C,Symbol.species,{configurable:true,enumerable:desc.enumerable,
          get:thrower('get '+name+'.@@Symbol(Symbol.species)')});
      }
      const targets=[[Array.prototype,'Array.prototype',['concat','filter','map','slice','splice']],
                     [Promise.prototype,'Promise.prototype',['then','catch','finally']]];
      for(const [o,label,members] of targets){
        for(const k of members){
          const d=gopd(o,k);if(!d||!d.configurable||typeof d.value!=='function')continue;
          saved.push([o,k,d]);
          def(o,k,{value:thrower(label+'.'+k),writable:true,configurable:true,enumerable:d.enumerable});
        }
      }
      const out={};
      try{
        document.body.appendChild(f);
        const w=f.contentWindow;
        out.contentWindow=!!(w && w.document);
        const captured=globalThis.__capturedSpecies||{};
        out.exposes={
          PromiseSpecies: captured.PromiseSpecies === Promise,
          ArraySpecies: captured.ArraySpecies === Array,
          MapSpecies: captured.MapSpecies === Map,
          SetSpecies: captured.SetSpecies === Set,
          RegExpSpecies: captured.RegExpSpecies === RegExp,
          ArrayBufferSpecies: captured.ArrayBufferSpecies === ArrayBuffer,
        };
      }catch(e){out.err=String(e.message||e)}
      finally{for(let i=saved.length-1;i>=0;i--){const [o,k,d]=saved[i];def(o,k,d)}}
      out.hooks=hooks;
      out.trips=trips;
      return JSON.stringify(out);
    })()"#,
    )
    .unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", thrown(catch.exception(), catch)));
    let got = result.to_rust_string_lossy(catch);
    assert!(
        got.contains("\"contentWindow\":true") && got.contains("\"hooks\":0"),
        "#449: iframe contentWindow must build without reading live species — got {got}"
    );
    assert!(
        got.contains("\"PromiseSpecies\":true")
            && got.contains("\"ArraySpecies\":true")
            && got.contains("\"MapSpecies\":true")
            && got.contains("\"SetSpecies\":true")
            && got.contains("\"RegExpSpecies\":true")
            && got.contains("\"ArrayBufferSpecies\":true"),
        "#449: the bootstrap must expose every `@@species` on `__tsNs` (<Ctor>Species) \
         so consumers can dispatch through a captured reference — got {got}"
    );
    reset();
}
