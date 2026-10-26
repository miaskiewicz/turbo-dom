//! DOM text mutation and text extraction using the native cascade's supported flow.
//! No geometric wrapping, generated content or composed/shadow layout is simulated.
use super::*;
use turbo_dom_parser::rtdom::cascade::{computed_style_with_policy, StylePolicy};

#[derive(Default)]
struct TextOutput {
    text: Vec<u16>,
    breaks: usize,
    space: bool,
}
impl TextOutput {
    fn separate(&mut self, count: usize) {
        self.space = false;
        self.breaks = self.breaks.max(count);
    }
    fn pending(&mut self) {
        if !self.text.is_empty() {
            for _ in 0..self.breaks {
                self.text.push(10);
            }
            if self.space && !self.text.last().is_some_and(|unit| matches!(unit, 10 | 9)) {
                self.text.push(32);
            }
        }
        self.breaks = 0;
        self.space = false;
    }
    fn normal(&mut self, value: &[u16], preserve_lines: bool) {
        for &character in value {
            if preserve_lines && character == 10 {
                self.forced('\n');
            } else if matches!(character, 9 | 10 | 13 | 12 | 32) {
                self.space = true;
            } else {
                self.pending();
                self.text.push(character);
            }
        }
    }
    fn preserved(&mut self, value: &[u16]) {
        if value.is_empty() {
            return;
        }
        self.pending();
        self.text.extend_from_slice(value);
    }
    fn forced(&mut self, value: char) {
        self.space = false;
        self.pending();
        let mut buffer = [0; 2];
        self.text.extend_from_slice(value.encode_utf16(&mut buffer));
    }
}
fn parent(tree: &Tree, node: Handle) -> Option<Handle> {
    tree.parent(node).or_else(|| tree.shadow_host(node))
}
pub(super) fn tag(tree: &Tree, node: Handle) -> &str {
    tree.local_name(node).unwrap_or("")
}
pub(super) fn display(tree: &Tree, node: Handle, style: &HashMap<String, String>) -> String {
    if let Some(value) = style.get("display") {
        return value.clone();
    }
    if tree.has_attribute(node, "hidden") {
        return "none".into();
    }
    match tag(tree, node) {
        "script" | "style" | "head" | "template" => "none",
        "td" | "th" => "table-cell",
        "tr" => "table-row",
        "table" => "table",
        "caption" => "table-caption",
        "p" | "div" | "section" | "article" | "aside" | "header" | "footer" | "main" | "nav"
        | "pre" | "blockquote" | "ul" | "ol" | "li" | "dl" | "dt" | "dd" | "figure"
        | "figcaption" | "form" | "fieldset" | "legend" | "details" | "summary" | "h1" | "h2"
        | "h3" | "h4" | "h5" | "h6" | "hr" => "block",
        _ => "inline",
    }
    .into()
}
pub(super) fn closed_details_child(tree: &Tree, node: Handle) -> bool {
    let Some(owner) = tree.parent(node) else {
        return false;
    };
    if tag(tree, owner) != "details" || tree.has_attribute(owner, "open") {
        return false;
    }
    tree.children(owner)
        .into_iter()
        .find(|child| tag(tree, *child) == "summary")
        != Some(node)
}
fn rendered(tree: &Tree, mut node: Handle, policy: &StylePolicy) -> bool {
    if !is_connected(tree, node) {
        return false;
    }
    loop {
        if tree.node_type_id(node) == 1
            && display(tree, node, &computed_style_with_policy(tree, node, policy)) == "none"
        {
            return false;
        }
        if closed_details_child(tree, node) {
            return false;
        }
        let Some(owner) = parent(tree, node) else {
            break;
        };
        node = owner;
    }
    true
}
pub(super) fn whitespace(tree: &Tree, mut node: Handle, style: &HashMap<String, String>) -> String {
    if let Some(value) = style.get("white-space") {
        return value.clone();
    }
    loop {
        if tag(tree, node) == "pre" {
            return "pre".into();
        }
        let Some(owner) = parent(tree, node) else {
            break;
        };
        node = owner;
    }
    "normal".into()
}
fn transform(units: &[u16], upper: bool) -> Vec<u16> {
    let mut out = Vec::new();
    for character in std::char::decode_utf16(units.iter().copied()) {
        match character {
            Ok(character) => {
                let text = if upper {
                    character.to_uppercase().collect::<String>()
                } else {
                    character.to_lowercase().collect::<String>()
                };
                out.extend(text.encode_utf16());
            }
            Err(error) => out.push(error.unpaired_surrogate()),
        }
    }
    out
}
fn collect(tree: &Tree, node: Handle, policy: &StylePolicy, out: &mut TextOutput) {
    let style = computed_style_with_policy(tree, node, policy);
    let kind = tree.node_type_id(node);
    if closed_details_child(tree, node) {
        return;
    }
    if kind == 3 {
        let Some(owner) = tree.parent(node) else {
            return;
        };
        let style = computed_style_with_policy(tree, owner, policy);
        if matches!(
            style.get("visibility").map(String::as_str),
            Some("hidden" | "collapse")
        ) {
            return;
        }
        let mut text = tree.node_value_utf16(node).unwrap_or_default();
        match style.get("text-transform").map(String::as_str) {
            Some("uppercase") => text = transform(&text, true),
            Some("lowercase") => text = transform(&text, false),
            _ => {}
        }
        match whitespace(tree, owner, &style).as_str() {
            "pre" | "pre-wrap" | "break-spaces" => out.preserved(&text),
            "pre-line" => out.normal(&text, true),
            _ => out.normal(&text, false),
        }
        return;
    }
    if kind != 1 {
        return;
    }
    let display = display(tree, node, &style);
    if display == "none" || closed_details_child(tree, node) {
        return;
    }
    let visible = !matches!(
        style.get("visibility").map(String::as_str),
        Some("hidden" | "collapse")
    );
    let name = tag(tree, node);
    if matches!(name, "input" | "textarea") {
        return;
    }
    if name == "br" {
        if visible {
            out.forced('\n');
        }
        return;
    }
    let block = matches!(
        display.as_str(),
        "block"
            | "flow-root"
            | "flex"
            | "grid"
            | "list-item"
            | "table"
            | "table-caption"
            | "table-row"
    ) || name == "option";
    let count = if name == "p" && block { 2 } else { 1 };
    if block {
        out.separate(count);
    }
    for child in tree.children(node) {
        collect(tree, child, policy, out);
    }
    if display == "table-cell" {
        if let Some(next) = tree.next_sibling(node) {
            let next_style = computed_style_with_policy(tree, next, policy);
            if display_for_cell(tree, next, &next_style) {
                out.forced('\t');
            }
        }
    }
    if block {
        out.separate(count);
    }
}
fn display_for_cell(tree: &Tree, node: Handle, style: &HashMap<String, String>) -> bool {
    display(tree, node, style) == "table-cell"
}
fn inner_text(tree: &Tree, node: Handle, policy: &StylePolicy) -> Vec<u16> {
    if !rendered(tree, node, policy) {
        return tree.text_content_utf16(node);
    }
    let mut out = TextOutput::default();
    for child in tree.children(node) {
        collect(tree, child, policy, &mut out);
    }
    out.text
}
fn replacement(tree: &mut Tree, text: &[u16]) -> Vec<Handle> {
    let mut normalized = Vec::new();
    let mut input = text.iter().copied().peekable();
    while let Some(unit) = input.next() {
        if unit == 13 {
            normalized.push(10);
            if input.peek() == Some(&10) {
                input.next();
            }
        } else {
            normalized.push(unit)
        }
    }
    let text = normalized;
    let mut nodes = Vec::new();
    for (index, line) in text.split(|&unit| unit == 10).enumerate() {
        if index > 0 {
            nodes.push(tree.create_element("br"));
        }
        if !line.is_empty() {
            nodes.push(tree.create_text_node_utf16(line));
        }
    }
    nodes
}
fn set_inner(tree: &mut Tree, node: Handle, text: &[u16]) {
    let nodes = replacement(tree, text);
    tree.replace_children(node, &nodes);
}
fn merge_text(scope: &mut v8::PinScope, first: Handle, second: Handle) {
    let Some((parent, mut value, suffix)) = with_tree(|tree| {
        Some((
            tree.parent(second)?,
            tree.node_value_utf16(first)?,
            tree.node_value_utf16(second)?,
        ))
    })
    .flatten() else {
        return;
    };
    range_live::replacing_data(scope, first, value.len(), 0, suffix.len());
    value.extend(suffix);
    range_live::removing(scope, second);
    with_tree_mut(|tree| {
        tree.set_text_content_utf16(first, &value);
        tree.remove_child(parent, second);
    });
}
fn set_outer(scope: &mut v8::PinScope, node: Handle, text: &[u16]) -> Result<(), &'static str> {
    let Some((parent, previous, next)) = with_tree(|tree| {
        Some((
            tree.parent(node)?,
            tree.previous_sibling(node),
            tree.next_sibling(node),
        ))
    })
    .flatten() else {
        return Err("NoModificationAllowedError");
    };
    let mut nodes = with_tree_mut(|tree| replacement(tree, text)).unwrap_or_default();
    if !dom_mutations::insert(scope, parent, &nodes, Some(node), &[node]) {
        return Ok(());
    }
    let is_text = |node| with_tree(|tree| tree.node_type_id(node) == 3).unwrap_or(false);
    if let Some(previous) = previous.filter(|node| is_text(*node)) {
        if let Some(following) = nodes
            .first()
            .copied()
            .or(next)
            .filter(|node| is_text(*node))
        {
            // outerText removes adjacent text after appending its data; unlike
            // normalize, ranges in the removed text collapse to the parent.
            merge_text(scope, previous, following);
            if !nodes.is_empty() {
                nodes[0] = previous;
            }
        }
    }
    if let (Some(last), Some(next)) = (nodes.last().copied(), next) {
        if last != next
            && with_tree(|tree| tree.parent(next) == Some(parent)).unwrap_or(false)
            && is_text(last)
            && is_text(next)
        {
            merge_text(scope, last, next);
        }
    }
    Ok(())
}
pub fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return type_error(scope);
    };
    let _dom = DomScope::for_object(scope, object);
    let Some(node) = handle_of(scope, object) else {
        return type_error(scope);
    };
    let valid = with_tree(|tree| {
        tree.node_type_id(node) == 1
            && tree.namespace(node) == turbo_dom_parser::rtdom::tree::Namespace::Html
    })
    .unwrap_or(false);
    if !valid {
        return type_error(scope);
    }
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        "check" => {}
        "read" => {
            let policy = css_policy::cascade_policy(scope);
            let value = with_tree(|tree| inner_text(tree, node, &policy)).unwrap();
            rv.set(unit_string(scope, &value).into());
        }
        "inner" => {
            let Some(value) =
                trusted_types::script_units(scope, object, args.get(2), "innerText", false)
            else {
                return;
            };
            html_popover::disconnect(scope, node, false);
            range_live::replacing_children(scope, node);
            with_tree_mut(|tree| set_inner(tree, node, &value));
        }
        "outer" => {
            let Some(value) = dom_units(scope, args.get(2)) else {
                return;
            };
            if with_tree(|tree| tree.parent(node).is_some()).unwrap_or(false) {
                html_popover::disconnect(scope, node, true);
            }
            if let Err(error) = set_outer(scope, node, &value) {
                rv.set(v8::String::new(scope, error).unwrap().into());
            }
        }
        _ => type_error(scope),
    }
}
fn type_error(scope: &mut v8::PinScope) {
    let message = v8::String::new(scope, "Illegal invocation").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(source: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        if result.is_none() {
            panic!(
                "HTML text fixture: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            );
        }
        assert!(result.unwrap().is_true());
        reset();
    }
    #[test]
    fn supported_rendered_text_flow_uses_native_css_visibility_whitespace_and_table_order() {
        fixture(
            r#"(()=>{
          const samples=[
            [' A  <span>B\t C</span> D ','A B C D'],
            ['first<div> second </div><div>third</div>end','first\nsecond\nthird\nend'],
            ['first<p> second </p><p>third</p>end','first\n\nsecond\n\nthird\n\nend'],
            ['first<br>second<br><br>','first\nsecond\n\n'],
            ['<pre> A  B\n C\tD </pre>end',' A  B\n C\tD \nend'],
            ["first<span hidden>hidden</span><span style='display:none'>gone</span>end",'firstend'],
            ["first<span style='visibility:hidden'>invisible<span style='visibility:visible'>visible</span></span>end",'firstvisibleend'],
            ["first<div style='display:contents'>second<br>third</div>end",'firstsecond\nthirdend'],
            ["first<span style='white-space:pre'>  A\n B </span>end",'first  A\n B end'],
            ['<table><tbody><tr><td> A </td><td> B </td></tr><tr><td>C</td><td>D</td></tr></tbody></table>end','A\tB\nC\tD\nend'],
            ["<span style='text-transform:uppercase'>mixed Case</span>",'MIXED CASE'],
            ["first<input value='input'><textarea>textarea</textarea><select><option>one</option><option>two</option></select>end",'first\none\ntwo\nend'],
            ['<details><summary>summary</summary>closed</details>end','summary\nend'],
            ['first<script>ignored</script><style>ignored {color:red}</style>end','firstend'],
          ];
          for(const [html,expected] of samples){const element=document.createElement('div');element.innerHTML=html;document.body.append(element);if(element.innerText!==expected)throw new Error(JSON.stringify({html,expected,actual:element.innerText}));element.remove();}
          const element=document.createElement('div');element.innerHTML=' A  <span hidden>hidden</span><br>B ';
          if(element.innerText!==' A  hiddenB ')throw new Error('disconnected fallback');document.body.append(element);element.style.display='none';if(element.innerText!==' A  hiddenB ')throw new Error('unrendered fallback');element.style.display='block';element.style.visibility='hidden';if(element.innerText!=='')throw new Error('visibility inherited');
          return true;
        })()"#,
        );
    }
    #[test]
    fn text_setters_normalize_newlines_escape_markup_detach_nodes_and_merge_neighbors() {
        fixture(
            r#"(()=>{
          const element=document.createElement('div');element.innerHTML='<b>old</b>';const retained=element.firstChild;document.body.append(element);
          element.innerText='a\r\nb\rc\n';if(element.innerHTML!=='a<br>b<br>c<br>'||element.textContent!=='abc'||element.innerText!=='a\nb\nc\n'||retained.parentNode!==null)throw new Error('newlines/detachment');
          element.innerText='<script>globalThis.exposed=true</script>';if(element.firstChild.nodeType!==3||element.childNodes.length!==1||globalThis.exposed)throw new Error('markup execution');
          element.innerText=null;if(element.childNodes.length)throw new Error('null');element.innerText=undefined;if(element.textContent!=='undefined')throw new Error('undefined');
          const parent=document.createElement('div');parent.innerHTML='left<span>old</span>right';document.body.append(parent);const old=parent.querySelector('span');old.outerText='A\nB';if(parent.innerHTML!=='leftA<br>Bright'||parent.childNodes.length!==3||old.parentNode!==null)throw new Error('outer merge');
          parent.innerHTML='left<span>old</span>right';parent.querySelector('span').outerText='';if(parent.textContent!=='leftright'||parent.childNodes.length!==1)throw new Error('empty merge');
          let error;try{old.outerText='x';}catch(caught){error=caught.name;}if(error!=='NoModificationAllowedError')throw new Error('detached outer');
          return true;
        })()"#,
        );
    }
    #[test]
    fn text_accessors_check_native_html_brands_before_coercion_and_ignore_guest_getters() {
        fixture(
            r#"(()=>{
          const getter=Object.getOwnPropertyDescriptor(HTMLElement.prototype,'innerText').get,setter=Object.getOwnPropertyDescriptor(HTMLElement.prototype,'innerText').set;let coerced=false;const errors=[];
          for(const receiver of [{},null,document.createElementNS('http://www.w3.org/2000/svg','svg')])for(const action of [()=>getter.call(receiver),()=>setter.call(receiver,{toString(){coerced=true;return 'bad'}})])try{action();errors.push('missing')}catch(error){errors.push(error.name)}
          if(coerced||errors.some(error=>error!=='TypeError'))throw new Error('brand/coercion');
          const element=document.createElement('div');element.innerHTML='<span>native</span>';document.body.append(element);Object.defineProperty(element,'textContent',{get(){throw new Error('guest');}});element.firstChild.getAttribute=()=>{throw new Error('guest')};if(getter.call(element)!=='native')throw new Error('guest getter consulted');
          let denied=false;try{setter.call(element,Symbol('x'));}catch(error){denied=error instanceof TypeError;}if(!denied||element.innerHTML!=='<span>native</span>')throw new Error('symbol mutated DOM');
          if('__htmlTextNative'in globalThis)throw new Error('native bridge leaked');return true;
        })()"#,
        );
    }
    #[test]
    fn popover_reflection_and_toggle_event_fields_use_private_native_brands() {
        fixture(
            r#"(()=>{
          const element=document.createElement('div'),values=[];
          for(const value of [null,'','auto','manual','hint','AUTO','bad',' auto ']){if(value===null)element.removeAttribute('popover');else element.setAttribute('popover',value);values.push(element.popover);}
          if(JSON.stringify(values)!==JSON.stringify([null,'auto','auto','manual','hint','auto','manual','manual']))throw new Error('enum reflection');
          element.popover='AUTO';if(element.getAttribute('popover')!=='AUTO'||element.popover!=='auto')throw new Error('author reflection');element.popover=null;if(element.hasAttribute('popover'))throw new Error('nullable reflection');
          const event=new ToggleEvent('sample',{oldState:'old',newState:'new',source:element,bubbles:true,cancelable:true});
          if(!(event instanceof Event)||!(event instanceof ToggleEvent)||event.oldState!=='old'||event.newState!=='new'||event.source!==element||event.isTrusted||!event.bubbles||!event.cancelable)throw new Error('event inheritance');
          if(Object.prototype.toString.call(event)!=='[object ToggleEvent]'||new ToggleEvent('empty').source!==null||new ToggleEvent('empty').oldState!=='')throw new Error('defaults/tag');
          Reflect.set(event,'oldState','changed');if(event.oldState!=='old')throw new Error('readonly state');
          const oldGet=WeakMap.prototype.get;WeakMap.prototype.get=function(){throw new Error('guest')};if(event.newState!=='new')throw new Error('private record');WeakMap.prototype.get=oldGet;
          const errors=[];for(const action of [()=>new ToggleEvent(),()=>ToggleEvent('x'),()=>new ToggleEvent('x',{source:{}}),()=>Object.getOwnPropertyDescriptor(ToggleEvent.prototype,'oldState').get.call({}),()=>Object.getOwnPropertyDescriptor(HTMLElement.prototype,'popover').set.call({},'auto'),()=>element.popover=Symbol('x')])try{action();errors.push('missing')}catch(error){errors.push(error.name)}
          if(errors.length!==6||errors.some(error=>error!=='TypeError'))throw new Error(JSON.stringify(errors));return true;
        })()"#,
        );
    }
}
