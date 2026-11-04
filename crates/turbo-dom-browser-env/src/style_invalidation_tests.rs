//! Subtree-scoped style invalidation (turbo-dom#279): a DOM mutation drops only the computed
//! styles it can change. Values against the Chrome 156 control
//! (`style_invalidation_probe.js` -> `style_invalidation_chrome156.json`), re-cascade counter
//! gates for reads after a mutation, and a randomised differential test of the reused styles
//! against a fresh cascade.

use super::run;

#[test]
fn style_invalidation_matches_the_chrome_control() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("style_invalidation_chrome156.json")).unwrap();
    let probe = include_str!("style_invalidation_probe.js")
        .trim()
        .trim_end_matches(';');
    let actual = run(
        "<!doctype html><html><head></head><body></body></html>",
        &format!("JSON.stringify({probe})"),
    );
    let (actual, expected) = (
        actual.as_object().unwrap(),
        expected["result"].as_object().unwrap(),
    );
    assert_eq!(actual.len(), expected.len());
    for (case, rows) in expected {
        assert_eq!(&actual[case], rows, "case {case}");
    }
}

/// A deep target under 30 divs and a mutated element elsewhere: `__gate(sheet, mutate)`
/// returns the fresh cascades of each target read after `mutate()`.
const GATE: &str = r#"
  globalThis.__gate=(mutate,reads=5)=>{
    let parent=document.body;
    for(let i=0;i<30;i++){const d=document.createElement('div');parent.appendChild(d);parent=d;}
    const target=document.createElement('p');target.className='t';parent.appendChild(target);
    const other=document.createElement('section');other.innerHTML='<b><i></i></b><b></b>';document.body.appendChild(other);
    getComputedStyle(target).color;
    const runs=[];
    for(let i=0;i<reads;i++){
      mutate(other,i);
      const before=__cssTestCascadeRuns();
      getComputedStyle(target).color;
      runs.push(__cssTestCascadeRuns()-before);
    }
    return {target,other,parent,max:Math.max(...runs)};
  };
"#;

fn gate(css: &str, code: &str) -> serde_json::Value {
    run(
        &format!("<style>{css}</style><body></body>"),
        &format!("(()=>{{{GATE}{code}}})()"),
    )
}

/// Class and attribute writes to an element outside the read element's ancestor chain
/// re-cascade nothing when the sheets have no sibling, structural or upward selectors.
#[test]
fn a_class_write_elsewhere_recascades_nothing() {
    let value = gate(
        ".t{color:red} .on .t{color:blue} [data-x] b{margin:1px}",
        r#"const g=__gate((other,i)=>{other.firstChild.className='k'+i;other.setAttribute('data-x',i);});
        return JSON.stringify({max:g.max,color:getComputedStyle(g.target).color});"#,
    );
    assert_eq!(value["color"], "rgb(255, 0, 0)");
    assert_eq!(value["max"], 0, "{value}");
}

/// Insertions and removals elsewhere re-cascade nothing either.
#[test]
fn a_structural_change_elsewhere_recascades_nothing() {
    let value = gate(
        ".t{color:red} section b{margin:1px}",
        r#"const g=__gate((other,i)=>{const n=document.createElement('b');other.prepend(n);other.lastChild.remove();n.textContent='x'+i;});
        return JSON.stringify({max:g.max});"#,
    );
    assert_eq!(value["max"], 0, "{value}");
}

/// A write to an ancestor re-cascades the chain below it (and the value follows).
#[test]
fn an_ancestor_write_recascades_its_subtree() {
    let value = gate(
        ".t{color:red} .on .t{color:blue}",
        r#"const g=__gate(()=>{},1);
        g.parent.classList.add('on');
        const before=__cssTestCascadeRuns();
        const color=getComputedStyle(g.target).color;
        const runs=__cssTestCascadeRuns()-before;
        return JSON.stringify({color,runs});"#,
    );
    assert_eq!(value["color"], "rgb(0, 0, 255)");
    let runs = value["runs"].as_u64().unwrap();
    assert!((1..=2).contains(&runs), "{value}");
}

/// With sibling selectors in the sheets, a write deep in another subtree still re-cascades
/// nothing, while a write to the read element's sibling is seen.
#[test]
fn sibling_selectors_widen_the_scope_to_the_child_lists_along_the_chain() {
    let value = gate(
        ".t{color:red} .on + .t{color:blue} p:nth-child(2){margin-left:3px}",
        r#"const g=__gate((other,i)=>{other.firstChild.firstChild.className='k'+i;});
        const sib=document.createElement('span');g.target.before(sib);
        const margin=getComputedStyle(g.target).marginLeft;
        sib.className='on';
        return JSON.stringify({max:g.max,margin,color:getComputedStyle(g.target).color});"#,
    );
    assert_eq!(value["max"], 0, "{value}");
    assert_eq!(value["margin"], "3px");
    assert_eq!(value["color"], "rgb(0, 0, 255)");
}

/// Sheets with only forward sibling selectors (`+`, `~`, `:nth-child`) keep every reused style
/// across an append to `<body>` (a portal), which no later sibling sees; `:last-child` makes
/// the append reach every element below `<body>`.
#[test]
fn an_append_reaches_only_backward_sibling_selectors() {
    let forward = gate(
        ".t{color:red} .a + .t{color:blue} div:nth-child(2) .t{margin-left:2px}",
        r#"const g=__gate(()=>{const n=document.createElement('aside');document.body.append(n);n.className='a';});
        return JSON.stringify({max:g.max});"#,
    );
    assert_eq!(forward["max"], 0, "{forward}");
    let backward = gate(
        ".t{color:red} body > div:last-child .t{color:blue}",
        r#"const g=__gate(()=>{document.body.append(document.createElement('aside'));},2);
        return JSON.stringify({max:g.max,color:getComputedStyle(g.target).color});"#,
    );
    assert_eq!(backward["color"], "rgb(255, 0, 0)");
    assert!(backward["max"].as_u64().unwrap() > 0, "{backward}");
}

/// `:invalid` (MUI's InputBase sheet) keys reused styles on form validity: class writes
/// re-cascade nothing, while a control's validity change is seen.
#[test]
fn form_validity_selectors_ignore_presentational_writes() {
    let value = gate(
        ".t{color:red} input:invalid{color:blue} form:invalid .t{margin-left:4px}",
        r#"const g=__gate((other,i)=>{other.firstChild.className='k'+i;});
        const form=document.createElement('form'),input=document.createElement('input'),t=document.createElement('p');
        t.className='t';form.append(input,t);document.body.append(form);
        const before=getComputedStyle(t).marginLeft+'|'+getComputedStyle(input).color;
        input.setAttribute('required','');
        const after=getComputedStyle(t).marginLeft+'|'+getComputedStyle(input).color;
        return JSON.stringify({max:g.max,before,after});"#,
    );
    assert_eq!(value["max"], 0, "{value}");
    // An unset margin-left computes to its initial 0px, like Chrome 156 (turbo-dom#305).
    assert_eq!(value["before"], "0px|rgb(0, 0, 0)");
    assert_eq!(value["after"], "4px|rgb(0, 0, 255)");
}

/// A write of a class, id or `aria-*` / `data-*` attribute no selector names moves no style
/// generation, even on an ancestor of the read element; a named class does.
#[test]
fn unnamed_class_and_attribute_writes_recascade_nothing() {
    let value = gate(
        ".t{color:red} .on .t{color:blue} [data-named] .t{margin-left:5px} #named .t{margin-left:6px}",
        r#"const g=__gate((other,i)=>{},1);
        const ancestor=g.parent.parentNode,before=__cssTestCascadeRuns();
        for(let i=0;i<5;i++){ancestor.className='x'+i;ancestor.setAttribute('aria-expanded',String(i%2));ancestor.dataset.other=i;ancestor.id='id'+i;getComputedStyle(g.target).color;}
        const unnamed=__cssTestCascadeRuns()-before;
        ancestor.classList.add('on');const on=getComputedStyle(g.target).color;
        ancestor.dataset.named='';const data=getComputedStyle(g.target).marginLeft;
        ancestor.id='named';const id=getComputedStyle(g.target).marginLeft;
        return JSON.stringify({unnamed,on,data,id});"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"unnamed":0,"on":"rgb(0, 0, 255)","data":"5px","id":"6px"})
    );
}

/// A <style> appended or prepended through the DOM (emotion's insertion) re-cascades only the
/// elements its rules can select (and their subtrees), like Chrome's rule-set invalidation;
/// the new rules apply, inside @media too, and before earlier sheets in the cascade order.
#[test]
fn an_added_sheet_recascades_only_what_it_selects() {
    let value = gate(
        ".t{color:red}",
        r#"globalThis.innerWidth=1024;
        const g=__gate(()=>{},1);
        const add=(css,first)=>{const s=document.createElement('style');s.textContent=css;first?document.head.prepend(s):document.head.append(s);};
        let before=__cssTestCascadeRuns();
        add('.css-a{margin-left:1px} @media (min-width: 1px){.css-b{margin-left:2px}} .css-b:hover{color:red}');
        const other=getComputedStyle(g.target).color, untouched=__cssTestCascadeRuns()-before;
        g.target.classList.add('css-b');
        const media=getComputedStyle(g.target).marginLeft;
        add('.t{color:blue}');
        const appended=getComputedStyle(g.target).color;
        add('.t{color:lime} .css-c ::placeholder{color:red}',true);
        const prepended=getComputedStyle(g.target).color;
        g.parent.classList.add('css-d');add('.css-d{color:rgb(1, 2, 3)} .css-d > .t{margin-left:3px}');
        return JSON.stringify({other,untouched,media,appended,prepended,inherited:getComputedStyle(g.parent).color,child:getComputedStyle(g.target).marginLeft});"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"other":"rgb(255, 0, 0)","untouched":0,"media":"2px","appended":"rgb(0, 0, 255)","prepended":"rgb(0, 0, 255)","inherited":"rgb(1, 2, 3)","child":"3px"})
    );
}

/// An animation starting or ending on another element re-cascades only that element's
/// subtree (MUI transitions animate their own nodes); the animated values still apply.
#[test]
fn an_animation_elsewhere_recascades_nothing() {
    let value = gate(
        ".t{color:red}",
        r#"const g=__gate((other,i)=>{const a=other.firstChild.animate([{opacity:0},{opacity:1}],{duration:1000});a.cancel();},3);
        const child=g.other.firstChild.firstChild;getComputedStyle(child).opacity;
        const a=g.other.firstChild.animate([{opacity:0.25},{opacity:0.25}],{duration:100000});
        const animated=getComputedStyle(g.other.firstChild).opacity;
        a.cancel();
        return JSON.stringify({max:g.max,animated,after:getComputedStyle(g.other.firstChild).opacity});"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"max":0,"animated":"0.25","after":"1"})
    );
}

/// `:has()` (and the other upward or document-wide dependencies) fall back to the tree-wide
/// version: a write anywhere is seen.
#[test]
fn has_falls_back_to_the_tree_wide_version() {
    let value = gate(
        ".t{color:red} body:has(.hit) .t{color:blue}",
        r#"const g=__gate((other,i)=>{other.firstChild.className='k'+i;},2);
        g.other.lastChild.className='hit';
        return JSON.stringify({max:g.max,color:getComputedStyle(g.target).color});"#,
    );
    assert_eq!(value["color"], "rgb(0, 0, 255)");
    assert!(value["max"].as_u64().unwrap() > 0, "{value}");
}

/// A stylesheet added, edited or removed through the DOM (not the CSSOM) reaches every
/// element, though the mutation's node scope does not contain them.
#[test]
fn sheet_changes_through_the_dom_reach_every_element() {
    let value = gate(
        ".t{color:red}",
        r#"const g=__gate(()=>{},1);
        const s=document.createElement('style');s.textContent='.t{color:blue}';document.head.appendChild(s);
        const added=getComputedStyle(g.target).color;
        s.textContent='.t{color:lime}';
        const edited=getComputedStyle(g.target).color;
        s.remove();
        return JSON.stringify({added,edited,removed:getComputedStyle(g.target).color});"#,
    );
    assert_eq!(
        value,
        serde_json::json!({"added":"rgb(0, 0, 255)","edited":"rgb(0, 255, 0)","removed":"rgb(255, 0, 0)"})
    );
}

/// Random documents, sheets and mutation sequences: after every mutation each element's
/// reused computed style equals a fresh cascade (`invalidateComputedStyles` drops every
/// reuse).
#[test]
fn reused_styles_equal_a_fresh_cascade_under_random_mutations() {
    let value = run(
        "<!doctype html><html><head></head><body></body></html>",
        r#"(()=>{
      let seed=279;const rnd=n=>{seed=(seed*1103515245+12345)&0x7fffffff;return seed%n;};
      const pick=a=>a[rnd(a.length)];
      const tags=['div','p','span','i','b','input','form','fieldset'],classes=['a','b','c','d','z','y'];
      const sels=['.c:last-child','li:nth-last-child(2) .a','input:invalid','form:invalid .b','.a .b','.a > .c','.b + .c','.c ~ .d','div:nth-child(2n)','span:first-child','p:last-of-type',
        '.d:only-child','.a:not(.b) i','[data-x] .c','#n3 .a',':is(.a,.d) > b','.b:empty','div:has(> .d)','.a *'];
      const props=['color','margin-left'];
      let mismatches=0,checks=0,first=null;
      for(let round=0;round<12;round++){
        const sheet=[];
        for(let r=0;r<5;r++){const s=pick(sels);sheet.push(r%2?`${s}{margin-left:${r+1}px}`:`${s}{color:rgb(${r*40}, 1, 2)}`);}
        if(round%4===3)sheet.pop();
        const extra=[];const style=document.createElement('style');style.textContent=sheet.filter(x=>!(round%3===0&&/:has|:empty/.test(x))).join(' ');document.head.append(style);
        const host=document.createElement('main');document.body.append(host);
        const all=[host];
        for(let n=0;n<25;n++){const el=document.createElement(pick(tags));if(rnd(2))el.className=pick(classes);el.id='n'+n;pick(all).append(el);all.push(el);}
        const ns=__cssTestNs;
        const read=()=>all.map(el=>props.map(p=>getComputedStyle(el).getPropertyValue(p)).join('|'));
        read();
        for(let step=0;step<40;step++){
          const el=pick(all.slice(1)),kind=rnd(7);
          if(kind===0)el.className=pick(classes);
          else if(kind===1)el.classList.toggle(pick(classes));
          else if(kind===2)el.toggleAttribute(pick(['data-x','required','data-q','aria-z']));else if(kind===6&&rnd(2))el.id=pick(['n3','n4','q']);
          else if(kind===3){const p=pick(all);if(!el.contains(p))p.insertBefore(el,rnd(2)?p.firstChild:null);}
          else if(kind===4)el.remove();
          else if(kind===5){if(!el.isConnected){const p=pick(all);if(!el.contains(p))p.append(el);}else el.append(document.createTextNode('t'));}
          else if(rnd(2))el.textContent='';
          else{const st=document.createElement('style');st.textContent=pick(sels)+(rnd(2)?'{color:rgb(9, 9, 9)}':'{margin-left:9px}');rnd(2)?document.head.prepend(st):document.head.append(st);extra.push(st);}
          if(rnd(3)===0)read();
          const reused=read();ns.invalidateComputedStyles();const fresh=read();
          for(let i=0;i<all.length;i++){checks++;if(reused[i]!==fresh[i]){mismatches++;first=first||{round,step,id:all[i].id,reused:reused[i],fresh:fresh[i]};}}
        }
        host.remove();style.remove();extra.forEach(x=>x.remove());
      }
      return JSON.stringify({mismatches,checks,first});
    })()"#,
    );
    assert_eq!(value["mismatches"], 0, "{value}");
    assert!(value["checks"].as_u64().unwrap() > 10_000, "{value}");
}
