//! Rule buckets for fresh cascades (turbo-dom#307): a cascade tests only the style rules whose
//! rightmost compound could select the element (every selector-list alternative names a class
//! or id the element carries, or names none). A randomised differential test checks the
//! cascade's matched rules against `Element.matches` for every element and rule, and counter
//! gates check the rules a cascade of a newly created element tests.

use super::run;

/// Every generated rule sets its own custom property over a universal rule's default, so the
/// cascade's verdict on each rule is visible in the computed style; the oracle is the native
/// `Element.matches` of the rule's selector.
#[test]
fn bucketed_cascade_matches_every_rule_the_selector_engine_matches() {
    let value = run(
        "<!doctype html><html><head></head><body></body></html>",
        r#"(()=>{
      // MINSTD, read from the high bits.
      let seed=307;const rand=n=>{seed=(seed*48271)%2147483647;return Math.floor(seed/2147483647*n);};
      const pick=list=>list[rand(list.length)];
      const classes=['a','b','c','A','d-1','é'],ids=['x','y','X'],tags=['div','span','p','section'];
      const simple=()=>pick([
        ()=>'.'+pick(classes),()=>'#'+pick(ids),()=>pick(tags),()=>'*',()=>'[data-k]',()=>'[data-k="1"]',
        ()=>'[class~="'+pick(classes)+'"]',()=>':first-child',()=>':not(.'+pick(classes)+')',
        ()=>':is(.'+pick(classes)+', #'+pick(ids)+')',()=>':where(.'+pick(classes)+')',()=>':nth-child(2n+1)',
        ()=>':has(> .'+pick(classes)+')',()=>'.'+pick(classes)+'.'+pick(classes),()=>'::before',()=>':root'])();
      const compound=()=>{let out='';const n=1+rand(3);for(let i=0;i<n;i++)out+=simple();return out;};
      const complex=()=>{let out=compound();const n=rand(3);for(let i=0;i<n;i++)out+=pick([' ',' > ',' + ',' ~ ','>'])+compound();return out;};
      const selector=()=>{let out=complex();if(rand(4)===0)out+=', '+complex();return out;};
      let checks=0,mismatches=[];
      for(let round=0;round<6;round++){
        document.head.textContent='';document.body.textContent='';
        const selectors=[];
        while(selectors.length<50){const text=selector();try{document.body.matches(text);}catch(e){continue;}selectors.push(text);}
        let css='* {'+selectors.map((_,i)=>'--r'+i+': n').join(';')+'}\n';
        selectors.forEach((text,i)=>{css+=text+' { --r'+i+': y }\n';});
        const style=document.createElement('style');style.textContent=css;document.head.appendChild(style);
        const all=[document.body];
        for(let i=0;i<40;i++){
          const el=document.createElement(pick(tags));
          const count=rand(4);const names=[];for(let k=0;k<count;k++)names.push(pick(classes));
          if(names.length)el.setAttribute('class',(rand(2)?'  ':'')+names.join(rand(2)?' ':'\t'));
          if(rand(3)===0)el.id=pick(ids);
          if(rand(3)===0)el.setAttribute('data-k',String(rand(2)));
          pick(all).appendChild(el);all.push(el);
        }
        for(const el of all){
          const computed=getComputedStyle(el);
          selectors.forEach((text,i)=>{
            checks++;
            const cascaded=computed.getPropertyValue('--r'+i).trim()==='y',matched=el.matches(text);
            if(cascaded!==matched&&mismatches.length<10)mismatches.push({text,element:el.outerHTML.slice(0,80),cascaded,matched});
          });
        }
      }
      return JSON.stringify({checks,mismatches});
    })()"#,
    );
    assert!(value["checks"].as_u64().unwrap() >= 12_000, "{value}");
    assert_eq!(value["mismatches"], serde_json::json!([]), "{value}");
}

/// A newly created element of an emotion-style page (one class rule per component) tests only
/// the rules keyed on its own classes plus the unkeyed ones, not every rule of the sheet.
#[test]
fn a_new_elements_cascade_tests_only_its_buckets() {
    let value = run(
        "<!doctype html><html><head></head><body></body></html>",
        r#"(()=>{
      let css='div { margin: 0 } *, *::before { box-sizing: border-box } body > p { padding: 1px } .multi-a, .multi-b { margin-top: 3px }\n';
      for(let i=0;i<400;i++)css+='.css-'+i+' { color: rgb('+(i%256)+', 0, 0) } .css-'+i+':hover { color: red } #id-'+i+' { padding: 2px }\n';
      const style=document.createElement('style');style.textContent=css;document.head.appendChild(style);
      const host=document.createElement('section');document.body.appendChild(host);
      getComputedStyle(host).color;
      const before=__cssTestRuleProbes(),colors=[];
      for(let i=0;i<20;i++){const d=document.createElement('div');d.className='css-'+(i*7)+' extra';host.appendChild(d);colors.push(getComputedStyle(d).color);}
      const probes=__cssTestRuleProbes()-before;
      const multi=document.createElement('div');multi.className='multi-a multi-b';host.appendChild(multi);
      const beforeMulti=__cssTestRuleProbes(),multiMargin=getComputedStyle(multi).marginTop,multiProbes=__cssTestRuleProbes()-beforeMulti;
      const keyed=document.createElement('p');keyed.id='id-5';keyed.className='css-5';document.body.appendChild(keyed);
      return JSON.stringify({probes,multiProbes,multiMargin,colors,keyed:[getComputedStyle(keyed).color,getComputedStyle(keyed).paddingTop]});
    })()"#,
    );
    assert_eq!(value["colors"][1], "rgb(7, 0, 0)", "{value}");
    assert_eq!(value["colors"][19], "rgb(133, 0, 0)", "{value}");
    assert_eq!(value["keyed"], serde_json::json!(["rgb(5, 0, 0)", "2px"]));
    // Per element: its two class rules and the three unkeyed rules.
    assert_eq!(value["probes"], 20 * 5, "{value}");
    // A rule keyed on two names the element carries is tested once.
    assert_eq!(value["multiMargin"], "3px");
    assert_eq!(value["multiProbes"], 4, "{value}");
}

/// Buckets fold case on both sides, so whichever case rule the selector engine applies to a
/// quirks-mode document's class and id selectors, the cascade agrees with `matches`.
#[test]
fn quirks_mode_class_and_id_buckets_ignore_case() {
    let value = run(
        "<html><head><style>.Foo { color: rgb(1, 2, 3) } #Bar { margin-top: 4px }</style></head><body></body></html>",
        r#"(()=>{
      const d=document.createElement('div');d.className='foo';d.id='bar';document.body.appendChild(d);
      const plain=document.createElement('div');document.body.appendChild(plain);
      return JSON.stringify({mode:document.compatMode,matched:d.matches('.Foo')&&d.matches('#Bar'),color:getComputedStyle(d).color,margin:getComputedStyle(d).marginTop,plain:[getComputedStyle(plain).color,getComputedStyle(plain).marginTop]});
    })()"#,
    );
    assert_eq!(value["mode"], "BackCompat");
    let expected = if value["matched"] == true {
        serde_json::json!(["rgb(1, 2, 3)", "4px"])
    } else {
        value["plain"].clone()
    };
    assert_eq!(
        serde_json::json!([value["color"], value["margin"]]),
        expected,
        "{value}"
    );
}
