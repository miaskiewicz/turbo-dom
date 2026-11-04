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
      let seed=3072;const rand=n=>{seed=(seed*1103515245+12345)%2147483648;return seed%n;};
      const pick=list=>list[rand(list.length)];
      const classes=['a','b','c','d','row','cell'],tags=['div','span','p','li','b','input','button','section'];
      const props=['display','position','color','font-size','font-weight','line-height','margin-top','padding-left','width','height','opacity','visibility','flex-grow','background-color','text-align','border-top-width','border-top-style','z-index','cursor','letter-spacing','font-family','--c','--d','zoom','content','float'];
      // Odd rounds also use pseudo-classes outside the style filter's scopes (:has, :empty,
      // :checked, :lang, :dir): their trees key on the tree-wide version and never share.
      let wide=false;
      const simple=()=>pick([()=>'.'+pick(classes),()=>pick(tags),()=>'[data-k]',()=>'[data-k="1"]',()=>':first-child',()=>':last-child',
        ()=>':nth-child(2n+1)',()=>':not(.'+pick(classes)+')',()=>':is(.'+pick(classes)+')',()=>':has(> .'+pick(classes)+')',()=>':empty',
        ()=>':checked',()=>':disabled',()=>':lang(pl)',()=>':dir(rtl)',()=>'[type=checkbox]',()=>'.'+pick(classes)+'.'+pick(classes),()=>':hover',()=>':only-child',()=>':disabled:not(.a)',()=>'.'+pick(classes)+':nth-child(2)'].filter(f=>wide||!/has|empty|checked|lang|dir\(/.test(f.toString())));
      const compound=()=>{let out=pick([()=>'.'+pick(classes),()=>pick(tags),()=>'']) ();const n=rand(3);for(let i=0;i<n;i++)out+=simple()();return out||'*';};
      const complex=()=>{let out=compound();const n=rand(3);for(let i=0;i<n;i++)out=compound()+pick([' ',' > ',' + ',' ~ '])+out;return out;};
      const decl=()=>pick(['color: rgb('+rand(256)+', 1, 2)','font-size: '+(1+rand(3))+'em','margin-top: '+rand(9)+'px','padding-left: '+rand(5)+'%',
        'display: '+pick(['flex','grid','block','inline','none','inline-block','contents']),'position: '+pick(['absolute','relative','static']),'opacity: 0.'+rand(10),
        'visibility: '+pick(['hidden','visible','inherit']),'flex-grow: '+rand(3),'background-color: #'+pick(['abc','123456','fff']),'font-weight: '+pick(['bold','400','inherit']),
        'line-height: '+pick(['1.5','normal','2em']),'text-align: '+pick(['center','start']),'border-top: '+rand(4)+'px solid red','z-index: '+rand(5),'cursor: pointer',
        'letter-spacing: '+rand(3)+'px','font-family: '+pick(['serif','a, b']),'--c: '+rand(9),'--d: var(--c)','zoom: '+pick(['1','2','normal']),'width: '+pick(['50%','10px','auto']),
        'float: '+pick(['left','none']),'height: '+pick(['3px','50%']),'color: inherit']);
      let checks=0,shares=0,mismatches=[];
      for(let round=0;round<10;round++){
        __cssTestStyleShareOff(false);wide=round%2===1;
        document.head.textContent='';document.body.textContent='';
        let css='';
        for(let i=0;i<40;i++){let text=complex();if(rand(4)===0)text+=', '+complex();try{document.body.matches(text);}catch(e){continue;}css+=text+' { '+decl()+'; '+decl()+(rand(5)===0?' !important':'')+' }\n';}
        if(rand(2))css+='@media (min-width: 1px) { .'+pick(classes)+' { margin-top: 7px } }\n';
        const style=document.createElement('style');style.textContent=css;document.head.appendChild(style);
        const parents=[document.body];
        for(let i=0;i<8;i++){const p=document.createElement(pick(['div','ul','section']));p.className=pick(classes)+(rand(2)?' '+pick(classes):'');if(rand(3)===0)p.style.display=pick(['flex','grid']);if(rand(4)===0)p.lang='pl';if(rand(5)===0)p.dir='rtl';pick(parents).appendChild(p);parents.push(p);}
        const groups=[];for(let i=0;i<6;i++){const tag=pick(tags);groups.push({tag,cls:pick(classes)+(rand(2)?' '+pick(classes):''),type:tag==='input'?pick(['checkbox','text']):null});}
        const all=[];
        const make=()=>{const g=pick(groups),el=document.createElement(g.tag);el.className=g.cls;if(g.type)el.type=g.type;
          if(rand(8)===0)el.setAttribute('data-k',String(rand(2)));if(rand(10)===0)el.style.marginTop=rand(9)+'px';if(rand(10)===0)el.lang='pl';
          if(g.type==='checkbox'&&rand(2))el.checked=true;if(g.tag==='button'&&rand(3)===0)el.disabled=true;if(rand(12)===0)el.textContent='t';return el;};
        for(const p of parents)for(let i=0,n=2+rand(7);i<n;i++){const el=make();p.appendChild(el);all.push(el);}
        all.push(...parents.slice(1));
        const read=()=>all.filter(el=>el.isConnected).map(el=>{const s=getComputedStyle(el);return props.map(name=>s.getPropertyValue(name));});
        const before=__cssTestStyleShares();
        let shared=read();
        if(round%3!==2){
          for(let i=0;i<12;i++){const el=pick(all);const op=rand(5);
            if(op===0)el.className=pick(groups).cls;else if(op===1)el.toggleAttribute('data-k');else if(op===2){const n=make();pick(parents).insertBefore(n,pick(parents).firstChild&&null);all.push(n);}
            else if(op===3&&el.parentNode&&parents.indexOf(el)<0)el.remove();else if(op===4){const n=make();if(el.parentNode&&parents.indexOf(el)<0){el.after(n);all.push(n);}}}
          shared=read();
        }
        shares+=__cssTestStyleShares()-before;
        __cssTestStyleShareOff(true);
        const bump=document.createElement('style');bump.textContent='.never-'+round+' { color: red }';document.head.appendChild(bump);
        const fresh=read();
        fresh.forEach((row,i)=>row.forEach((v,j)=>{checks++;if(v!==shared[i][j]&&mismatches.length<10)mismatches.push({round,prop:props[j],shared:shared[i][j],fresh:v,css:css.length});}));
      }
      __cssTestStyleShareOff(false);
      return JSON.stringify({checks,shares,mismatches});
    })()"#,
    );
    assert!(value["checks"].as_u64().unwrap() >= 10_000, "{value}");
    assert!(value["shares"].as_u64().unwrap() >= 100, "{value}");
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
