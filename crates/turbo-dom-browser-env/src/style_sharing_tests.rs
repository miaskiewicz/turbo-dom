//! Style sharing and the transition base cache (turbo-dom#307 part 2, turbo-test#227 part 2).
//!
//! A new element whose sharing signature matches a recently cascaded sibling's takes that
//! sibling's cascaded declarations instead of matching rules again; an element with a running
//! transition or animation reuses its cascaded (non-animated) declarations across reads and only
//! recomputes the animated values. A randomised differential test checks shared computed styles
//! against fresh cascades (sharing switched off), and counter gates check the rules tested.

use super::run;

/// Random sheets (classes, ids, attributes, structural and state pseudo-classes, sibling
/// combinators, `:has`, `:lang`, flex / grid parents, `em` sizes, inheritance, custom
/// properties) over random sibling groups (shared class lists, form controls, inline styles,
/// `lang` / `dir` / data attributes). Every round reads every element with sharing on, mutates
/// part of the tree (class and attribute writes, insertions, removals) and reads again with
/// sharing on; the oracle then reads the same tree with sharing off after a CSSOM change that
/// drops every reused style. Rounds without a mutation compare the first reads.
#[test]
fn shared_styles_match_fresh_cascades() {
    let value = run(
        "<!doctype html><html><head></head><body></body></html>",
        r#"(()=>{
      // MINSTD, read from the high bits.
      let seed=3072;const rand=n=>{seed=(seed*48271)%2147483647;return Math.floor(seed/2147483647*n);};
      const pick=list=>list[rand(list.length)];
      const classes=['a','b','c','d','row','cell'],tags=['div','span','p','li','b','input','button','section'];
      const props=['display','position','color','font-size','font-weight','line-height','margin-top','padding-left','width','height','opacity','visibility','flex-grow','background-color','text-align','border-top-width','border-top-style','z-index','cursor','letter-spacing','font-family','--c','--d','zoom','content','float'];
      // Every fourth round also uses pseudo-classes outside the style filter's scopes (:has, :empty,
      // :checked, :lang, :dir): their trees key on the tree-wide version and never share.
      let wide=false;
      // Mostly signature-only compounds (classes, tags, attributes), so most rules are decided
      // by the signature alone; the rest test position, state or siblings.
      const plain=()=>pick([()=>'.'+pick(classes),()=>'.'+pick(classes),()=>'.'+pick(classes),()=>pick(tags),()=>'[data-k]',()=>'[data-k="1"]',()=>'[lang]',()=>'#x1',()=>'[type=checkbox]']);
      const simple=()=>rand(3)?plain():pick([
        ()=>':first-child',()=>':last-child',()=>':nth-child(2n+1)',()=>':not(.'+pick(classes)+')',()=>':is(.'+pick(classes)+')',()=>':hover',()=>':only-child',
        ()=>'[type=checkbox]',()=>':is(button, input):disabled',()=>'.'+pick(classes)+':nth-child(2)',
        ()=>':has(> .'+pick(classes)+')',()=>':empty',()=>':checked',()=>':lang(pl)',()=>':dir(rtl)'].filter(f=>wide||!/has|empty|checked|lang\(|dir\(/.test(f.toString())));
      const compound=()=>{let out=pick([()=>'.'+pick(classes),()=>'.'+pick(classes),()=>pick(tags)])();const n=rand(3);for(let i=0;i<n;i++)out+=simple()();return out;};
      const combinator=()=>rand(4)?pick([' ',' > ']):pick([' + ',' ~ ']);
      const complex=()=>{let out=compound();const n=rand(3);for(let i=0;i<n;i++)out=compound()+combinator()+out;return out;};
      const decl=()=>pick(['color: rgb('+rand(256)+', 1, 2)','font-size: '+(1+rand(3))+'em','margin-top: '+rand(9)+'px','padding-left: '+rand(5)+'%',
        'display: '+pick(['flex','grid','block','inline','none','inline-block','contents']),'position: '+pick(['absolute','relative','static']),'opacity: 0.'+rand(10),
        'visibility: '+pick(['hidden','visible','inherit']),'flex-grow: '+rand(3),'background-color: #'+pick(['abc','123456','fff']),'font-weight: '+pick(['bold','400','inherit']),
        'line-height: '+pick(['1.5','normal','2em']),'text-align: '+pick(['center','start']),'border-top: '+rand(4)+'px solid red','z-index: '+rand(5),'cursor: pointer',
        'letter-spacing: '+rand(3)+'px','font-family: '+pick(['serif','a, b']),'--c: '+rand(9),'--d: var(--c)','zoom: '+pick(['1','2','normal']),'width: '+pick(['50%','10px','auto']),
        'float: '+pick(['left','none']),'height: '+pick(['3px','50%']),'color: inherit']);
      let checks=0,shares=0,offShares=0,stale=0,mismatches=[];
      for(let round=0;round<20;round++){
        __cssTestStyleShareOff(false);wide=round%4===3;
        document.head.textContent='';document.body.textContent='';
        let css='';
        for(let i=0;i<40;i++){let text=complex();if(rand(4)===0)text+=', '+complex();try{document.body.matches(text);}catch(e){continue;}css+=text+' { '+decl()+'; '+decl()+(rand(5)===0?' !important':'')+' }\n';}
        if(rand(2))css+='@media (min-width: 1px) { .'+pick(classes)+' { margin-top: 7px } }\n';
        const style=document.createElement('style');style.textContent=css;document.head.appendChild(style);
        const parents=[document.body];
        for(let i=0;i<8;i++){const p=document.createElement(pick(['div','ul','section']));p.className=pick(classes)+(rand(2)?' '+pick(classes):'');if(rand(3)===0)p.style.display=pick(['flex','grid']);if(rand(4)===0)p.lang='pl';if(rand(5)===0)p.dir='rtl';pick(parents).appendChild(p);parents.push(p);}
        // Few tags and class lists, so many siblings share a signature and many differ from a
        // sharing sibling in one input only (class, id, data-k, lang, type, position, state).
        const lists=['a','b','a b','row cell'];
        const all=[];
        const make=()=>{const tag=pick(['div','div','div','div','span','li','input','button']),el=document.createElement(tag);el.className=pick(lists);
          if(tag==='input')el.type=pick(['checkbox','text']);
          if(rand(8)===0)el.setAttribute('data-k',String(rand(2)));if(rand(12)===0)el.style.marginTop=rand(9)+'px';if(rand(15)===0)el.lang='pl';if(rand(15)===0)el.id='x'+rand(2);
          if(el.type==='checkbox'&&rand(2))el.checked=true;if(tag==='button'&&rand(3)===0)el.disabled=true;if(rand(12)===0)el.textContent='t';return el;};
        for(const p of parents)for(let i=0,n=4+rand(9);i<n;i++){const el=make();p.appendChild(el);all.push(el);}
        all.push(...parents.slice(1));
        const read=()=>all.filter(el=>el.isConnected).map(el=>{const s=getComputedStyle(el);return props.map(name=>s.getPropertyValue(name));});
        const before=__cssTestStyleShares();
        let shared=read();
        if(round%3!==2){
          for(let i=0;i<12;i++){const el=pick(all);const op=rand(5);
            if(op===0)el.className=pick(lists);else if(op===1)el.toggleAttribute('data-k');else if(op===2){const n=make();pick(parents).insertBefore(n,pick(parents).firstChild&&null);all.push(n);}
            else if(op===3&&el.parentNode&&parents.indexOf(el)<0)el.remove();else if(op===4){const n=make();if(el.parentNode&&parents.indexOf(el)<0){el.after(n);all.push(n);}}}
          shared=read();
        }
        shares+=__cssTestStyleShares()-before;
        __cssTestStyleShareOff(true);
        const bump=document.createElement('style');document.head.appendChild(bump);bump.sheet.insertRule('.never-'+round+' { color: red }',0);
        const runsBefore=__cssTestCascadeRuns();
        const offBefore=__cssTestStyleShares();
        const fresh=read();
        offShares+=__cssTestStyleShares()-offBefore;
        if(__cssTestCascadeRuns()-runsBefore<fresh.length)stale++;
        fresh.forEach((row,i)=>row.forEach((v,j)=>{checks++;if(v!==shared[i][j]&&mismatches.length<10)mismatches.push({round,prop:props[j],shared:shared[i][j],fresh:v,css:css.length});}));
      }
      __cssTestStyleShareOff(false);
      return JSON.stringify({checks,shares,offShares,stale,mismatches});
    })()"#,
    );
    assert!(value["checks"].as_u64().unwrap() >= 10_000, "{value}");
    assert!(value["shares"].as_u64().unwrap() >= 150, "{value}");
    assert_eq!(value["offShares"], 0, "{value}");
    // The oracle cascades every element afresh.
    assert_eq!(value["stale"], 0, "{value}");
    assert_eq!(value["mismatches"], serde_json::json!([]), "{value}");
}

/// Twenty new siblings with one class list cascade once: the other nineteen share the first
/// one's declarations and test no rule.
#[test]
fn same_signature_siblings_test_rules_once() {
    let value = run(
        "<!doctype html><html><head></head><body></body></html>",
        r#"(()=>{
      let css='div { margin: 0 } *, *::before { box-sizing: border-box } body > p { padding: 1px }\n';
      for(let i=0;i<400;i++)css+='.css-'+i+' { color: rgb('+(i%256)+', 0, 0) } .css-'+i+':hover { color: red }\n';
      const style=document.createElement('style');style.textContent=css;document.head.appendChild(style);
      const host=document.createElement('section');document.body.appendChild(host);getComputedStyle(host).color;
      const rows=[];for(let i=0;i<20;i++){const d=document.createElement('div');d.className='css-7 extra';host.appendChild(d);rows.push(d);}
      const probes=__cssTestRuleProbes(),shares=__cssTestStyleShares();
      const colors=rows.map(d=>getComputedStyle(d).color);
      return JSON.stringify({probes:__cssTestRuleProbes()-probes,shares:__cssTestStyleShares()-shares,colors:[...new Set(colors)]});
    })()"#,
    );
    // One cascade: its class rules (2) and the three unkeyed rules.
    assert_eq!(value["probes"], 5, "{value}");
    assert_eq!(value["shares"], 19, "{value}");
    assert_eq!(value["colors"], serde_json::json!(["rgb(7, 0, 0)"]));
}

/// An element with a running transition re-reads its animated values on every read, but its
/// cascaded declarations are reused: no read after the first tests a rule. The reported value
/// stays the transition's start value (Chrome 156).
#[test]
fn a_running_transition_reuses_its_cascaded_declarations() {
    let value = run(
        "<!doctype html><html><head><style>.v { transition: opacity 100s linear; opacity: 1; color: rgb(1, 2, 3) } .v.dim { opacity: 0.5 }</style></head><body><div class=v id=v></div></body>",
        r#"(()=>{
      const v=document.getElementById('v');getComputedStyle(v).opacity;v.classList.add('dim');
      const first=getComputedStyle(v).opacity,probes=__cssTestRuleProbes(),values=[];
      for(let i=0;i<20;i++){const s=getComputedStyle(v);values.push(s.opacity+'/'+s.color);}
      return JSON.stringify({first,probes:__cssTestRuleProbes()-probes,values:[...new Set(values)],running:v.getAnimations().length});
    })()"#,
    );
    assert_eq!(value["first"], "1");
    assert_eq!(value["probes"], 0, "{value}");
    assert_eq!(value["values"], serde_json::json!(["1/rgb(1, 2, 3)"]));
    assert_eq!(value["running"], 1);
}


/// Runs `body` (which returns the values to compare) with sharing on, then again over the same
/// tree with sharing off after a CSSOM change that drops every reused style.
fn shared_and_fresh(html: &str, setup: &str, read: &str) -> serde_json::Value {
    let code = format!(
        r#"(()=>{{{setup}
      const read=()=>{{{read}}};
      const before=__cssTestStyleShares(),shared=read(),shares=__cssTestStyleShares()-before;
      __cssTestStyleShareOff(true);
      const bump=document.createElement('style');document.head.appendChild(bump);bump.sheet.insertRule('.never {{ color: red }}',0);
      const fresh=read();__cssTestStyleShareOff(false);
      return JSON.stringify({{shared,fresh,shares}});
    }})()"#
    );
    run(html, &code)
}

/// A sibling cascaded before its parent changed is no source: the parent's style generation
/// moved, so a new child cascades with the parent's new class.
#[test]
fn a_parent_change_ends_sharing_with_earlier_siblings() {
    let value = shared_and_fresh(
        "<!doctype html><html><head><style>.x { color: rgb(1, 1, 1) } .open > .x { color: rgb(2, 2, 2) }</style></head><body><section id=p></section></body>",
        "const p=document.getElementById('p'),first=document.createElement('div');first.className='x';p.appendChild(first);getComputedStyle(first).color;p.className='open';const late=document.createElement('div');late.className='x';p.appendChild(late);",
        "return getComputedStyle(late).color;",
    );
    assert_eq!(value["shared"], "rgb(2, 2, 2)", "{value}");
    assert_eq!(value["fresh"], value["shared"]);
}

/// Form controls never share: their UA tables read the `type` attribute even when no sheet
/// names it.
#[test]
fn form_controls_never_share() {
    let value = shared_and_fresh(
        "<!doctype html><html><head><style>.f { color: rgb(1, 1, 1) }</style></head><body><form id=p></form></body>",
        "const p=document.getElementById('p'),a=document.createElement('input'),b=document.createElement('input'),c=document.createElement('button'),d=document.createElement('button');a.className=b.className=c.className=d.className='f';b.type='date';d.type='submit';p.append(a,b,c,d);",
        "return [a,b,c,d].map(el=>{const s=getComputedStyle(el);return [s.fontFamily,s.fontSize,s.display,s.paddingTop].join('/');});",
    );
    assert_eq!(value["shares"], 0, "{value}");
    assert_eq!(value["fresh"], value["shared"], "{value}");
    assert_ne!(value["shared"][0], value["shared"][1], "{value}");
}

/// A sibling whose state rule cannot match the source but can match it (relaxed: the state
/// pseudo-class taken as matching) cascades itself, so its style follows that state later.
#[test]
fn a_possible_state_match_cascades_itself() {
    let value = run(
        "<!doctype html><html><head><style>.a { color: rgb(1, 1, 1) } .a:first-child:popover-open { color: rgb(2, 2, 2) }</style></head><body><section id=p><div></div></section></body>",
        r#"(()=>{
      const p=document.getElementById('p'),source=document.createElement('div');source.className='a';source.popover='manual';p.appendChild(source);getComputedStyle(source).color;
      p.firstElementChild.remove();
      const late=document.createElement('div');late.className='a';late.popover='manual';p.prepend(late);
      const before=getComputedStyle(late).color;late.showPopover();
      return JSON.stringify([before,getComputedStyle(late).color]);
    })()"#,
    );
    assert_eq!(value, serde_json::json!(["rgb(1, 1, 1)", "rgb(2, 2, 2)"]));
}

/// A dialog with a transition rule that becomes modal cascades again (the modal UA table).
#[test]
fn a_transitioning_element_entering_the_top_layer_cascades_again() {
    let value = run(
        "<!doctype html><html><head><style>.v { transition: opacity 100s linear; opacity: 1 } .v.dim { opacity: 0.5 }</style></head><body><dialog class=v id=v>d</dialog></body>",
        r#"(()=>{
      const v=document.getElementById('v');getComputedStyle(v).opacity;v.classList.add('dim');
      const before=getComputedStyle(v);const closed=[before.position,before.display];
      v.showModal();const open=getComputedStyle(v);
      return JSON.stringify({closed,open:[open.position,open.display,open.opacity]});
    })()"#,
    );
    assert_eq!(value["closed"], serde_json::json!(["absolute", "none"]));
    assert_eq!(value["open"], serde_json::json!(["fixed", "block", "0.5"]));
}
