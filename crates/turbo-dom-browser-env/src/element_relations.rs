//! Invalidate explicit ARIA references on native attribute mutation, including Attr writes.
use super::*;

pub(super) fn attribute_node_changed(scope: &mut v8::PinScope, node: Handle) {
    let owner = with_tree(|tree| {
        let data = tree.attribute_data(node)?;
        if data.namespace.is_some() {
            return None;
        }
        Some((tree.attribute_owner(node)?, data.local_name.clone()))
    })
    .flatten();
    if let Some((owner, name)) = owner {
        let owner = wrap(scope, owner);
        attribute_changed(scope, owner, &name);
    }
}

pub(super) fn attribute_changed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    element: v8::Local<'s, v8::Object>,
    name: &str,
) {
    if matches!(name, "disabled" | "form" | "id") {
        element_internals::attribute_changed(scope, element, name);
    }
    file_inputs::attribute_changed(scope, element, name);
    if let Some(handle) = handle_of(scope, element) {
        script_state::attribute_changed(handle, name);
        form_checked_state::attribute_changed(handle, name);
        user_activation_gates::attribute_changed(scope, handle, name);
    }
    if name == "interestfor" {
        interest_invokers::attribute_changed(scope, element);
    }
    if name == "type" {
        selection::type_changed(scope, element);
    }
    if !name.starts_with("aria-") && !matches!(name, "popovertarget" | "commandfor" | "interestfor")
    {
        return;
    }
    let Some(record) = collection_records::get(scope, element)
        .filter(|value| value.is_object())
        .and_then(|value| value.to_object(scope))
    else {
        return;
    };
    let key = v8::String::new(scope, "aria").unwrap();
    if !record.has_own_property(scope, key.into()).unwrap_or(false) {
        return;
    }
    let Some(references) = record
        .get(scope, key.into())
        .and_then(|value| value.to_object(scope))
    else {
        return;
    };
    let key = v8::String::new(scope, name).unwrap();
    references.delete(scope, key.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_contract(source: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(scope, source).unwrap();
        v8::tc_scope!(let scope, scope);
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let Some(result) = result else {
            let key = v8::String::new(scope, "stack").unwrap();
            panic!(
                "contract threw: {}",
                scope
                    .exception()
                    .and_then(|value| value.to_object(scope))
                    .and_then(|value| value.get(scope, key.into()))
                    .map(|value| value.to_rust_string_lossy(scope))
                    .unwrap_or_default()
            );
        };
        let value: serde_json::Value =
            serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
        for (name, value) in value.as_object().unwrap() {
            assert_eq!(*value, true, "{name}: {value}");
        }
        reset();
    }

    #[test]
    fn token_lists_have_cached_native_brands_live_indices_and_atomic_validation() {
        check_contract(
            r#"(()=>{
          const el=document.createElement('div');el.className=' a a\tb  c ';const list=el.classList;
          const identity=list===el.classList&&list instanceof DOMTokenList&&Object.prototype.toString.call(list)==='[object DOMTokenList]';
          const indices=Object.keys(list).join(',')==='0,1,2'&&list.length===3&&list[1]==='b'&&list.item(99)===null;
          const raw=list.value===' a a\tb  c '&&String(list)===el.className;
          let atomic=false,empty=false,whitespace=false,supports=false,brand=false;
          try{list.add('new','a b')}catch(e){whitespace=e.name==='InvalidCharacterError';atomic=el.className===' a a\tb  c ';}
          try{list.remove('')}catch(e){empty=e.name==='SyntaxError';}
          try{list.supports('a')}catch(e){supports=e instanceof TypeError;}
          try{DOMTokenList.prototype.add.call({},'a')}catch(e){brand=e instanceof TypeError;}
          const contains=!list.contains('')&&!list.contains('a b');
          const forced=list.toggle('b',undefined)===false&&list.value==='a c';
          const replace=list.replace('a','c')&&list.value==='c'&&!list.replace('missing','next');
          list.value='a b c';list.replace('a','c');const replaceOrder=list.value==='c b';
          el.classList='x x\ny';const assigned=list.value==='x x\ny'&&Array.from(list).join(',')==='x,y';
          el.setAttribute('class','later');const live=list[0]==='later'&&list.length===1;
          el.part='one two';const part=el.part===el.part&&el.part instanceof DOMTokenList&&el.getAttribute('part')==='one two';
          let argumentsValid=false;try{list.toggle()}catch(e){argumentsValid=e instanceof TypeError;}
          let third=true;list.forEach((value,index,owner)=>{third=third&&owner===list&&owner[index]===value;});
          const keys=Array.from(list.keys()).join(',')==='0'&&Array.from(list.entries()).map(pair=>pair.join(':')).join(',')==='0:later';
          return JSON.stringify({identity,indices,raw,atomic,empty,whitespace,supports,brand,contains,forced,replace,replaceOrder,assigned,live,part,argumentsValid,third,keys});
        })()"#,
        );
    }

    #[test]
    fn aria_references_use_frozen_caches_and_native_attribute_invalidation() {
        check_contract(
            r#"(()=>{
          const el=document.createElement('div'),a=document.createElement('span'),b=document.createElement('span');a.id='a';b.id='b';document.body.append(el,a,b);
          const absent=el.ariaControlsElements===null&&el.ariaActiveDescendantElement===null;
          el.setAttribute('aria-controls','a missing b a');const refs=el.ariaControlsElements;
          const attribute=refs.map(node=>node.id).join(',')==='a,b,a'&&refs===el.ariaControlsElements&&Object.isFrozen(refs);
          const assigned=[b,a,b];el.ariaControlsElements=assigned;assigned.length=0;
          const explicit=el.getAttribute('aria-controls')===''&&el.ariaControlsElements.map(node=>node.id).join(',')==='b,a';
          el.setAttribute('aria-controls','');const reset=el.ariaControlsElements.length===0;
          el.ariaControlsElements=[a];el.getAttributeNode('aria-controls').value='';const attrValue=el.ariaControlsElements.length===0;
          el.ariaControlsElements=[a];el.getAttributeNode('aria-controls').nodeValue='';const nodeValue=el.ariaControlsElements.length===0;
          el.ariaControlsElements=[a];el.getAttributeNode('aria-controls').textContent='';const textContent=el.ariaControlsElements.length===0;
          el.ariaControlsElements=[a];document.adoptNode(el.getAttributeNode('aria-controls'));const attrAdopt=el.ariaControlsElements===null;
          el.ariaControlsElements=[a];el.setAttributeNS(null,'aria-controls','');const namespace=el.ariaControlsElements.length===0;
          el.ariaControlsElements=[a];const replacement=document.createAttribute('aria-controls');replacement.value='b';el.setAttributeNode(replacement);const attrNode=el.ariaControlsElements[0]===b;
          el.ariaControlsElements=null;const cleared=el.ariaControlsElements===null&&el.getAttribute('aria-controls')===null;
          let invalid=false;try{el.ariaControlsElements=[a,{}]}catch(e){invalid=e instanceof TypeError&&el.getAttribute('aria-controls')===null;}
          el.ariaActiveDescendantElement=b;const single=el.ariaActiveDescendantElement===b&&el.getAttribute('aria-activedescendant')==='';
          el.setAttribute('aria-activedescendant','a');const resetSingle=el.ariaActiveDescendantElement===a;
          a.remove();const live=el.ariaActiveDescendantElement===null;
          const aliases=['ariaActionsElements','ariaDescribedByElements','ariaDetailsElements','ariaErrorMessageElements','ariaFlowToElements','ariaLabelledByElements'].every(name=>{el[name]=[b];return el[name][0]===b&&Object.isFrozen(el[name]);});
          return JSON.stringify({absent,attribute,explicit,reset,attrValue,nodeValue,textContent,attrAdopt,namespace,attrNode,cleared,invalid,single,resetSingle,live,aliases});
        })()"#,
        );
    }

    #[test]
    fn aria_reference_scope_tracks_shadow_roots_detachment_and_reconnection() {
        check_contract(
            r#"(()=>{
          const parent=document.createElement('div'),target=document.createElement('span'),host=document.createElement('div');target.id='target';document.body.append(parent,target,host);
          const shadow=host.attachShadow({mode:'open'}),child=document.createElement('div'),inside=document.createElement('i');inside.id='inside';shadow.append(child,inside);
          child.ariaControlsElements=[target,inside];const ancestors=child.ariaControlsElements[0]===target&&child.ariaControlsElements[1]===inside;
          parent.ariaControlsElements=[inside];const descendant=parent.ariaControlsElements.length===0;
          child.setAttribute('aria-controls','target inside');const attributes=child.ariaControlsElements.length===1&&child.ariaControlsElements[0]===inside;
          const detached=document.createElement('b');parent.ariaControlsElements=[detached];const filtered=parent.ariaControlsElements.length===0;
          document.body.append(detached);const reconnected=parent.ariaControlsElements[0]===detached;
          detached.remove();const removed=parent.ariaControlsElements.length===0;
          child.ariaActiveDescendantElement=target;const oneAncestor=child.ariaActiveDescendantElement===target;
          parent.ariaActiveDescendantElement=inside;const oneDescendant=parent.ariaActiveDescendantElement===null;
          return JSON.stringify({ancestors,descendant,attributes,filtered,reconnected,removed,oneAncestor,oneDescendant});
        })()"#,
        );
    }
}
