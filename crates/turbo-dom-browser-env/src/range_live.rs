//! Live boundaries follow native tree mutations without retaining collected ranges.
//!
//! Selection editor records (Chromium's `SelectionEditor` anchor/focus positions) share the
//! registry but follow Blink's own position rules: node insertion never shifts them, a text
//! split only moves positions inside the split node, and character-data changes that move
//! them notify the owning selection.
use super::*;
use std::rc::Rc;

struct Entry {
    record: v8::Weak<v8::Object>,
    /// Present for selection editor records: called after a data/split/merge change moved them.
    editor: Option<v8::Weak<v8::Function>>,
}

#[derive(Default)]
struct Records(RefCell<Vec<Entry>>);

#[derive(Clone, Copy, PartialEq)]
enum Change {
    Insert,
    Remove,
    Data,
    Split,
}

fn records(scope: &mut v8::PinScope) -> Rc<Records> {
    scope.get_slot::<Rc<Records>>().cloned().unwrap_or_else(|| {
        let records = Rc::new(Records::default());
        scope.set_slot(records.clone());
        records
    })
}

pub(super) fn register(scope: &mut v8::PinScope, record: v8::Local<v8::Object>) {
    let weak = v8::Weak::new(scope, record);
    records(scope).0.borrow_mut().push(Entry {
        record: weak,
        editor: None,
    });
}

/// Track a selection editor record whose start pair is the anchor and end pair the focus.
pub(super) fn register_editor(
    scope: &mut v8::PinScope,
    record: v8::Local<v8::Object>,
    notify: v8::Local<v8::Function>,
) {
    let weak = v8::Weak::new(scope, record);
    // The notifier is retained by its document's ownership graph, never by this registry.
    let Some(notify) = dom_ownership::helper_function(scope, notify) else {
        return;
    };
    records(scope).0.borrow_mut().push(Entry {
        record: weak,
        editor: Some(notify),
    });
}

pub(super) fn clear(isolate: &mut v8::Isolate) {
    isolate.remove_slot::<Rc<Records>>();
    selection::clear();
}

fn update(
    scope: &mut v8::PinScope,
    kind: Change,
    change: impl Fn(Handle, usize, bool) -> Option<(Handle, usize)>,
) {
    let Some(records) = scope.get_slot::<Rc<Records>>().cloned() else {
        return;
    };
    let mut notify = Vec::new();
    {
        let mut entries = records.0.borrow_mut();
        entries.retain(|entry| !entry.record.is_empty());
        for entry in entries.iter() {
            let editor = entry.editor.is_some();
            if editor && kind == Change::Insert {
                continue;
            }
            let Some(record) = entry.record.to_local(scope) else {
                continue;
            };
            let mut moved = false;
            for (container, offset) in [
                ("startContainer", "startOffset"),
                ("endContainer", "endOffset"),
            ] {
                let Some(node) = get_prop(scope, record, container)
                    .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                else {
                    continue;
                };
                if dom_id_of(scope, node) != DOM_ID.with(|id| id.get()) {
                    continue;
                }
                let Some(handle) = handle_of(scope, node) else {
                    continue;
                };
                let position = get_prop(scope, record, offset)
                    .and_then(|value| value.uint32_value(scope))
                    .unwrap_or(0) as usize;
                if let Some((node, position)) = change(handle, position, editor) {
                    let node = wrap(scope, node);
                    let position = v8::Number::new(scope, position as f64);
                    set_prop(scope, record, container, node.into());
                    set_prop(scope, record, offset, position.into());
                    moved = true;
                }
            }
            if moved && kind != Change::Remove {
                if let Some(callback) = entry.editor.as_ref().and_then(|weak| weak.to_local(scope))
                {
                    notify.push(v8::Global::new(scope, callback));
                }
            }
        }
    }
    for callback in notify {
        let callback = v8::Local::new(scope, callback);
        let receiver = v8::undefined(scope);
        callback.call(scope, receiver.into(), &[]);
    }
}

pub(super) fn removing(scope: &mut v8::PinScope, node: Handle) {
    let Some((parent, index)) = with_tree(|tree| {
        let parent = tree.parent(node)?;
        let index = tree
            .children(parent)
            .iter()
            .position(|&child| child == node)?;
        Some((parent, index))
    })
    .flatten() else {
        return;
    };
    selection::textarea_children(scope, parent, false);
    selection::removing(scope, node);
    update(scope, Change::Remove, |container, offset, _| {
        let descendant = with_tree(|tree| {
            let mut current = Some(container);
            while let Some(handle) = current {
                if handle == node {
                    return true;
                }
                current = tree.parent(handle);
            }
            false
        })
        .unwrap_or(false);
        if descendant {
            Some((parent, index))
        } else if container == parent && offset > index {
            Some((parent, offset - 1))
        } else {
            None
        }
    });
}

pub(super) fn inserting(scope: &mut v8::PinScope, parent: Handle, index: usize, count: usize) {
    update(scope, Change::Insert, |container, offset, _| {
        (container == parent && offset > index).then_some((container, offset + count))
    });
}

pub(super) fn replacing_data(
    scope: &mut v8::PinScope,
    node: Handle,
    offset: usize,
    count: usize,
    inserted: usize,
) {
    if let Some(parent) = with_tree(|tree| tree.parent(node)).flatten() {
        selection::textarea_children(scope, parent, false);
    }
    update(scope, Change::Data, |container, position, _| {
        if container != node || position <= offset {
            None
        } else if position <= offset.saturating_add(count) {
            Some((container, offset))
        } else {
            Some((container, position - count + inserted))
        }
    });
}

pub(super) fn splitting(scope: &mut v8::PinScope, node: Handle, next: Handle, offset: usize) {
    let parent = with_tree(|tree| {
        let parent = tree.parent(node)?;
        let index = tree
            .children(parent)
            .iter()
            .position(|&child| child == node)?;
        Some((parent, index))
    })
    .flatten();
    // Blink's editor moves only positions inside the split node; parent offsets are insertions.
    update(scope, Change::Split, |container, position, editor| {
        if container == node && position > offset {
            Some((next, position - offset))
        } else if !editor
            && parent.is_some_and(|(parent, index)| container == parent && position > index)
        {
            Some((container, position + 1))
        } else {
            None
        }
    });
}

pub(super) fn replacing_children(scope: &mut v8::PinScope, node: Handle) {
    selection::textarea_children(scope, node, true);
    let children = with_tree(|tree| tree.children(node)).unwrap_or_default();
    // Reverse order keeps each removal's sibling index valid until the bulk edit.
    for child in children.into_iter().rev() {
        removing(scope, child);
    }
}

pub(super) fn merging_text(scope: &mut v8::PinScope, into: Handle, from: Handle, length: usize) {
    let position = with_tree(|tree| {
        let parent = tree.parent(from)?;
        let index = tree
            .children(parent)
            .iter()
            .position(|&node| node == from)?;
        Some((parent, index))
    })
    .flatten();
    update(scope, Change::Data, |container, offset, _| {
        if container == from {
            Some((into, offset + length))
        } else if position == Some((container, offset)) {
            Some((into, length))
        } else {
            None
        }
    });
}

pub(super) fn normalize(scope: &mut v8::PinScope, root: Handle) {
    let mut parents = vec![root];
    while let Some(parent) = parents.pop() {
        let mut node = with_tree(|tree| tree.first_child(parent)).flatten();
        while let Some(current) = node {
            if scope.is_execution_terminating() {
                return;
            }
            if with_tree(|tree| tree.node_type_id(current)) != Some(3) {
                parents.push(current);
                node = with_tree(|tree| tree.next_sibling(current)).flatten();
                continue;
            }
            let mut text = with_tree(|tree| tree.node_value_utf16(current))
                .flatten()
                .unwrap_or_default();
            if text.is_empty() {
                node = with_tree(|tree| tree.next_sibling(current)).flatten();
                removing(scope, current);
                with_tree_mut(|tree| tree.remove_child(parent, current));
                continue;
            }
            while let Some(next) = with_tree(|tree| tree.next_sibling(current)).flatten() {
                if scope.is_execution_terminating() {
                    return;
                }
                if with_tree(|tree| tree.node_type_id(next)) != Some(3) {
                    break;
                }
                let length = text.len();
                text.extend(
                    with_tree(|tree| tree.node_value_utf16(next))
                        .flatten()
                        .unwrap_or_default(),
                );
                merging_text(scope, current, next, length);
                with_tree_mut(|tree| tree.set_text_content_utf16(current, &text));
                removing(scope, next);
                with_tree_mut(|tree| tree.remove_child(parent, next));
            }
            node = with_tree(|tree| tree.next_sibling(current)).flatten();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_boundaries_match_independent_chrome_mutation_control() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body></body>");
        let source = format!(
            "JSON.stringify({})",
            include_str!("range_live_probe.js")
                .trim()
                .trim_end_matches(';')
        );
        let source = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope, scope);
        let value = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let value = value.unwrap_or_else(|| {
            panic!(
                "Live Range collector: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            )
        });
        let actual: serde_json::Value =
            serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("range_live_chrome155.json")).unwrap();
        assert_eq!(actual, expected["result"]);
        reset();
    }

    #[test]
    fn private_boundaries_ignore_guest_getters() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body><p>abcdef</p></body>");
        let source = v8::String::new(scope, r#"(()=>{
          const node=document.querySelector('p'),text=node.firstChild,range=document.createRange();range.setStart(text,2);range.setEnd(text,5);
          let hooks=0;const poison=()=>{hooks++;throw new Error('guest boundary hook')};
          for(const key of ['startContainer','endContainer','startOffset','endOffset'])Object.defineProperty(range,key,{get:poison,configurable:true});
          for(const key of ['parentNode','childNodes'])Object.defineProperty(text,key,{get:poison,configurable:true});
          text.insertData(1,'ZZ');node.removeChild(text);
          for(const key of ['startContainer','endContainer','startOffset','endOffset'])delete range[key];
          return hooks===0&&range.startContainer===node&&range.endContainer===node&&range.startOffset===0&&range.endOffset===0;
        })()"#).unwrap();
        v8::tc_scope!(let scope, scope);
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        assert!(
            result.is_some_and(|result| result.is_true()),
            "Private Range boundary update failed"
        );
        reset();
    }
    #[test]
    fn live_registry_does_not_retain_unused_boundary_records() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body></body>");
        run_js(
            scope,
            "(()=>{for(let i=0;i<100;i++)document.createRange();})()",
        );
        let records = scope.get_slot::<Rc<Records>>().cloned().unwrap();
        assert!(records
            .0
            .borrow()
            .iter()
            .any(|entry| !entry.record.is_empty()));
        scope.low_memory_notification();
        scope.low_memory_notification();
        // Selection editor records stay alive with the document's live Selection; only the
        // unused createRange boundary records must be released.
        assert!(records
            .0
            .borrow()
            .iter()
            .filter(|entry| entry.editor.is_none())
            .all(|entry| entry.record.is_empty()));
        reset();
    }

    #[test]
    fn borrowed_range_boundaries_follow_the_associated_foreign_tree() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let main = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, main);
        install_with_html(scope, "<!doctype html><body><p>main</p></body>");
        let foreign = v8::Context::new(scope, Default::default());
        let foreign_range = {
            let scope = &mut v8::ContextScope::new(scope, foreign);
            install_independent_html(scope, "<!doctype html><body><p>abcdef</p></body>");
            let source = v8::String::new(scope, "(()=>{const r=document.createRange();r.setStart(document.querySelector('p').firstChild,2);r.setEnd(r.startContainer,5);return r;})()").unwrap();
            let range = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Global::new(scope, range)
        };
        let range = v8::Local::new(scope, foreign_range);
        let key = v8::String::new(scope, "foreignRange").unwrap();
        main.global(scope).set(scope, key.into(), range);
        let source = v8::String::new(scope, "(()=>{const own=document.createRange();own.selectNodeContents(document.querySelector('p').firstChild);const clone=Range.prototype.cloneRange.call(foreignRange);foreignRange.startContainer.insertData(1,'ZZ');return JSON.stringify({clone:[clone.startOffset,clone.endOffset,String(clone)],foreign:[foreignRange.startOffset,foreignRange.endOffset],own:String(own)});})()").unwrap();
        v8::tc_scope!(let scope, scope);
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let result = result.unwrap_or_else(|| {
            panic!(
                "Foreign Range error: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            )
        });
        let actual: serde_json::Value =
            serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
        assert_eq!(
            actual,
            serde_json::json!({"clone":[4,7,"cde"],"foreign":[4,7],"own":"main"})
        );
        reset();
    }
}
