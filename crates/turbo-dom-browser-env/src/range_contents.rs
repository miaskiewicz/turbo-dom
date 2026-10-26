//! Private Range accessor installation; all tree operations use captured DOM methods.

struct RangeAccessor(v8::Weak<v8::Function>);
struct RangeExceptionUpdater(v8::Weak<v8::Function>);

fn contextual_fragment<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    let Ok(start) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let _owner = super::DomScope::for_object(scope, start);
    let Some(start_handle) = super::handle_of(scope, start) else {
        return;
    };
    let Some(source) = args.get(1).to_string(scope) else {
        return;
    };
    let units = super::string_code_units(scope, source);
    let scripting_enabled = super::node_has_window_owner(scope, start);
    let Some((context, fragment)) = super::with_tree_mut(|tree| {
        let context = if tree.node_type_id(start_handle) == 1 {
            Some(start_handle)
        } else {
            tree.parent(start_handle)
                .filter(|node| tree.node_type_id(*node) == 1)
        };
        let context = context
            .filter(|node| tree.namespace_id(*node) != 0 || tree.local_name(*node) != Some("html"));
        // The HTML-root and non-element cases use a body parser context. This
        // private native node never runs a page custom-element constructor.
        let context = context.unwrap_or_else(|| tree.create_element("body"));
        let fragment = tree.create_contextual_fragment_utf16(context, &units, scripting_enabled);
        (context, fragment)
    }) else {
        return;
    };
    let context = super::wrap(scope, context);
    let fragment = super::wrap(scope, fragment);
    if let Some(content) = super::template_contents::ensure(scope, context) {
        let target = {
            let _owner = super::DomScope::for_object(scope, content);
            let Some(root) = super::with_tree(super::Tree::root) else {
                return;
            };
            super::wrap(scope, root)
        };
        if super::document_basics::adopt(scope, target, fragment).is_err() {
            return;
        }
    }
    super::template_contents::realize(scope, fragment);
    super::template_contents::mark_parsed(scope, fragment, false);
    super::custom_elements::parsed_fragment(scope, fragment, context);
    result.set(fragment.into());
}

// Static boundaries own their nodes through the wrapper's private V8 edge. They
// deliberately never enter the live-boundary mutation registry.
fn static_record(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let name = v8::String::new(scope, "turbo-dom.StaticRange.record").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    let receiver = if args.get(0).is_undefined() && args.get(1).is_object() {
        let template = v8::ObjectTemplate::new(scope);
        template.set_internal_field_count(1);
        let object = template.new_instance(scope).unwrap();
        object.set_prototype(scope, args.get(2));
        Some(object)
    } else {
        v8::Local::<v8::Object>::try_from(args.get(0)).ok()
    };
    if let Some(receiver) = receiver {
        if args.get(1).is_object() {
            receiver.set_private(scope, key, args.get(1));
            rv.set(receiver.into());
        } else if let Some(record) = receiver.get_private(scope, key) {
            rv.set(record);
        }
    }
}

pub(super) fn capture(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    let Some(name) = v8::String::new(scope, "__nativeRangeRecord") else {
        return;
    };
    let Some(function) = global
        .get(scope, name.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let Some(accessor) = super::dom_ownership::helper_function(scope, function) else {
        return;
    };
    scope
        .get_current_context()
        .set_slot(std::rc::Rc::new(RangeAccessor(accessor)));
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let Some(accessor) = scope.get_current_context().get_slot::<RangeAccessor>() else {
        return;
    };
    let accessor = super::dom_ownership::local(scope, &accessor.0);
    let global = scope.get_current_context().global(scope);
    let Some(name) = v8::String::new(scope, "ts") else {
        return;
    };
    let key = v8::Symbol::for_key(scope, name);
    let Some(namespace) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let Some(name) = v8::String::new(scope, "makeNativeFunction") else {
        return;
    };
    let Some(factory) = namespace.get(scope, name.into()) else {
        return;
    };
    let Some(source) = v8::String::new(scope, include_str!("range_contents.js")) else {
        return;
    };
    if let Some(initializer) = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let convert = v8::Function::new(scope, super::trusted_types::convert_call).unwrap();
        let static_accessor = v8::Function::new(scope, static_record).unwrap();
        let parser = v8::Function::new(scope, contextual_fragment).unwrap();
        let receiver = v8::undefined(scope);
        if let Some(updater) = initializer
            .call(
                scope,
                receiver.into(),
                &[
                    accessor.into(),
                    factory,
                    convert.into(),
                    static_accessor.into(),
                    parser.into(),
                ],
            )
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            let Some(updater) = super::dom_ownership::helper_function(scope, updater) else {
                return;
            };
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(RangeExceptionUpdater(updater)));
        }
    }
}

pub(super) fn capture_interfaces(scope: &mut v8::PinScope) {
    let Some(updater) = scope
        .get_current_context()
        .get_slot::<RangeExceptionUpdater>()
    else {
        return;
    };
    let function = super::dom_ownership::local(scope, &updater.0);
    let global = scope.get_current_context().global(scope);
    let Some(name) = v8::String::new(scope, "DOMException") else {
        return;
    };
    if let Some(constructor) = global.get(scope, name.into()) {
        let receiver = v8::undefined(scope);
        function.call(scope, receiver.into(), &[constructor]);
    }
}

/// Private Range record accessors for the selection module: (live record, static record).
pub(super) fn accessors<'s>(
    scope: &mut v8::PinScope<'s, '_>,
) -> Option<(v8::Local<'s, v8::Function>, v8::Local<'s, v8::Function>)> {
    let accessor = scope.get_current_context().get_slot::<RangeAccessor>()?;
    let accessor = super::dom_ownership::local(scope, &accessor.0);
    let static_accessor = v8::Function::new(scope, static_record)?;
    Some((accessor, static_accessor))
}

/// Install the selection's Range::UpdateSelectionIfAddedToSelection hook.
pub(super) fn set_selection_sync(scope: &mut v8::PinScope, hook: v8::Local<v8::Function>) {
    let Some(updater) = scope
        .get_current_context()
        .get_slot::<RangeExceptionUpdater>()
    else {
        return;
    };
    let function = super::dom_ownership::local(scope, &updater.0);
    let receiver = v8::undefined(scope);
    let constructor = v8::undefined(scope);
    function.call(scope, receiver.into(), &[constructor.into(), hook.into()]);
}

pub(super) fn clear_context(context: v8::Local<v8::Context>) {
    context.remove_slot::<RangeAccessor>();
    context.remove_slot::<RangeExceptionUpdater>();
}

#[cfg(test)]
mod tests {
    use super::super::{
        capture_window_event_interfaces, install_with_html, parse_html_document, reset, run_js,
    };

    #[test]
    fn abstract_node_and_static_ranges_match_chrome_boundaries_and_ownership() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("abstract_ranges_chrome155.json")).unwrap();
        let probe = include_str!("abstract_ranges_probe.js")
            .trim()
            .trim_end_matches(';');
        let expected = serde_json::to_string(&fixture["result"]).unwrap();
        let source = format!("(()=>{{const actual=({probe});const expected={expected};const normalize=value=>Array.isArray(value)?value.map(normalize):value&&typeof value===\"object\"?Object.fromEntries(Object.keys(value).sort().map(key=>[key,normalize(value[key])])):value;if(JSON.stringify(normalize(actual))!==JSON.stringify(normalize(expected)))throw new Error(JSON.stringify(actual));return true;}})()");
        check_script(&source, None);
    }

    #[test]
    fn borrowed_static_range_accessors_keep_foreign_native_boundary_owners() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let main = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, main);
        install_with_html(scope, "<!doctype html><body>main</body>");
        let foreign = v8::Context::new(scope, Default::default());
        let foreign_range = {
            let child_scope = &mut v8::ContextScope::new(scope, foreign);
            super::super::install_independent_html(
                child_scope,
                "<!doctype html><body>foreign</body>",
            );
            let code = v8::String::new(child_scope, "new StaticRange({startContainer:document.body.firstChild,startOffset:2,endContainer:document.body.firstChild,endOffset:5})").unwrap();
            let value = v8::Script::compile(child_scope, code, None)
                .unwrap()
                .run(child_scope)
                .unwrap();
            v8::Global::new(child_scope, value)
        };
        let foreign_range = v8::Local::new(scope, foreign_range);
        let key = v8::String::new(scope, "foreignStaticRange").unwrap();
        main.global(scope).set(scope, key.into(), foreign_range);
        let code = v8::String::new(scope, "(()=>{const r=foreignStaticRange,get=Object.getOwnPropertyDescriptor(NodeRange.prototype,'startContainer').get;const node=get.call(r);node.deleteData(0,3);return JSON.stringify([r.startOffset,r.endOffset,node.data,r.collapsed,Object.getPrototypeOf(r)!==StaticRange.prototype]);})()").unwrap();
        let result = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            result.to_rust_string_lossy(scope),
            "[2,5,\"eign\",false,true]"
        );
        reset();
    }

    fn check_script(source: &str, refresh: Option<&str>) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body></body>");
        if let Some(setup) = refresh {
            run_js(scope, setup);
            capture_window_event_interfaces(scope);
        }
        let source = v8::String::new(scope, source).unwrap();
        v8::tc_scope!(let scope,scope);
        let value = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        assert!(
            value.is_some_and(|value| value.is_true()),
            "Range contract failed: {:?}",
            scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope))
        );
        reset();
    }

    #[test]
    fn range_errors_use_the_private_final_exception_constructor() {
        check_script(
            r#"(()=>{const range=document.createRange();let hooks=0;globalThis.DOMException=function(){hooks++;throw new Error('guest constructor')};let error;try{range.setStart(document.doctype,0)}catch(caught){error=caught}return hooks===0&&error instanceof FinalException&&error.name==='InvalidNodeTypeError'})()"#,
            Some("globalThis.FinalException=class FinalException extends DOMException{};globalThis.DOMException=FinalException;"),
        );
    }

    #[test]
    fn range_content_operations_bypass_replaced_dom_methods_and_own_getters() {
        check_script(
            r#"(()=>{
          const host=document.createElement('p');host.textContent='abcdef';document.body.append(host);
          const text=host.firstChild,range=document.createRange(),wrapper=document.createElement('mark');range.setStart(text,1);range.setEnd(text,4);
          let hooks=0;const poison=()=>{hooks++;throw new Error('guest DOM hook')};
          const saved=[];for(const [proto,names]of [[Node.prototype,['cloneNode','appendChild','removeChild','insertBefore']],[Text.prototype,['splitText','deleteData']]])for(const name of names){saved.push([proto,name,Object.getOwnPropertyDescriptor(proto,name)]);Object.defineProperty(proto,name,{value:poison,writable:true,configurable:true})}
          Object.defineProperty(text,'data',{get:poison,configurable:true});Object.defineProperty(text,'parentNode',{get:poison,configurable:true});
          let clone,wrapped;try{clone=range.cloneContents().textContent;range.surroundContents(wrapper);wrapped=wrapper.textContent}finally{delete text.data;delete text.parentNode;for(const [proto,name,descriptor]of saved){if(descriptor)Object.defineProperty(proto,name,descriptor);else delete proto[name]}}
          return hooks===0&&clone==='bcd'&&wrapped==='bcd'&&host.innerHTML==='a<mark>bcd</mark>ef';
        })()"#,
            None,
        );
    }

    #[test]
    fn invalid_range_insertion_does_not_split_text_before_hierarchy_validation() {
        check_script(
            r#"(()=>{
          const host=document.createElement('p');host.textContent='abcdef';document.body.append(host);const text=host.firstChild,range=document.createRange();range.setStart(text,2);range.collapse(true);
          let error;try{range.insertNode(document)}catch(caught){error=caught.name}
          const detached=document.createTextNode('detached'),other=document.createRange();other.selectNodeContents(detached);
          return error==='HierarchyRequestError'&&host.childNodes.length===1&&text.data==='abcdef'&&range.startContainer===text&&range.startOffset===2&&other.cloneContents().textContent==='detached';
        })()"#,
            None,
        );
    }

    #[test]
    fn range_contents_and_boundaries_match_authored_chrome155_control() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        super::super::install_with_html(scope, "<!doctype html><body></body>");
        // The standalone binding lacks DOMImplementation; inject a genuine
        // independently parsed document for the control's cross-tree queries.
        let foreign = parse_html_document(scope, "<!doctype html><body></body>", false);
        let key = v8::String::new(scope, "__rangeForeignDoc").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), foreign.into());
        let probe = include_str!("range_probe.js").replace(
            "document.implementation.createHTMLDocument(\"foreign\")",
            "__rangeForeignDoc",
        );
        let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
        let source = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope,scope);
        let value = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let value = value.unwrap_or_else(|| {
            panic!(
                "Range control failed: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            )
        });
        let actual: serde_json::Value =
            serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("range_chrome155.json")).unwrap();
        assert_eq!(actual, expected["result"]);
        reset();
    }
}
