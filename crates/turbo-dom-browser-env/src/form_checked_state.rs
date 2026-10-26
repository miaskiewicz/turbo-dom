//! Native radio grouping keeps state on Tree handles, independently of wrappers.
use super::*;
use turbo_dom_parser::rtdom::tree::FormProp;

pub(super) fn checked(node: Handle) -> bool {
    with_tree(|tree| {
        tree.form_property(node, FormProp::Checked)
            .unwrap_or_else(|| tree.get_attribute(node, "checked").is_some())
    })
    .unwrap_or(false)
}
fn radio(tree: &Tree, node: Handle) -> bool {
    tree.namespace_id(node) == 0
        && tree.local_name(node) == Some("input")
        && tree
            .get_attribute(node, "type")
            .is_some_and(|value| value.eq_ignore_ascii_case("radio"))
}
fn descendants(root: Handle) -> Vec<Handle> {
    with_tree(|tree| {
        let mut result = Vec::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            result.push(node);
            pending.extend(tree.children(node).into_iter().rev());
        }
        result
    })
    .unwrap_or_default()
}
fn candidates() -> Vec<Handle> {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .map(|state| state.checked_radios.keys().copied().collect())
            .unwrap_or_default()
    })
}
pub(super) fn refresh(node: Handle) {
    let selected = with_tree(|tree| {
        tree.is_live(node)
            && radio(tree, node)
            && tree
                .form_property(node, FormProp::Checked)
                .unwrap_or_else(|| tree.get_attribute(node, "checked").is_some())
    })
    .unwrap_or(false);
    let owner = if selected { closest_form(node) } else { None };
    DOM.with(|dom| {
        if let Some(state) = dom.borrow_mut().as_mut() {
            if selected {
                state.checked_radios.insert(node, owner);
            } else {
                state.checked_radios.remove(&node);
            }
        }
    });
}
fn root(tree: &Tree, mut node: Handle) -> Handle {
    while let Some(parent) = tree.parent(node) {
        node = parent;
    }
    node
}
fn within(tree: &Tree, ancestor: Handle, mut node: Handle) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        let Some(parent) = tree.parent(node).or_else(|| tree.shadow_host(node)) else {
            return false;
        };
        node = parent;
    }
}
fn path(tree: &Tree, ancestor: Handle, mut node: Handle) -> Option<Vec<usize>> {
    let mut result = Vec::new();
    while node != ancestor {
        let parent = tree.parent(node)?;
        result.push(
            tree.children(parent)
                .iter()
                .position(|child| *child == node)?,
        );
        node = parent;
    }
    result.reverse();
    Some(result)
}
pub(super) fn peers(node: Handle) -> Vec<Handle> {
    let Some((group_root, name)) = with_tree(|tree| {
        if !radio(tree, node) {
            return None;
        }
        let name = tree.get_attribute_utf16(node, "name")?;
        if name.is_empty() {
            return None;
        }
        Some((root(tree, node), name))
    })
    .flatten() else {
        return Vec::new();
    };
    let owner = closest_form(node);
    if owner.is_none() && !with_tree(|tree| is_connected(tree, node)).unwrap_or(false) {
        return Vec::new();
    }
    candidates()
        .into_iter()
        .filter(|peer| {
            *peer != node
                && with_tree(|tree| {
                    tree.is_live(*peer)
                        && radio(tree, *peer)
                        && root(tree, *peer) == group_root
                        && tree.get_attribute_utf16(*peer, "name").as_ref() == Some(&name)
                })
                .unwrap_or(false)
                && closest_form(*peer) == owner
        })
        .collect()
}
pub(super) fn group(node: Handle) {
    refresh(node);
    if !checked(node) {
        return;
    }
    let peers = peers(node)
        .into_iter()
        .filter(|peer| checked(*peer))
        .collect::<Vec<_>>();
    with_tree_mut(|tree| {
        for &peer in &peers {
            tree.set_form_property(peer, FormProp::Checked, false);
        }
    });
    for peer in peers {
        refresh(peer);
    }
}
pub(super) fn set(node: Handle, value: bool) {
    with_tree_mut(|tree| tree.set_form_property(node, FormProp::Checked, value));
    group(node);
}
pub(super) fn attribute_changed(node: Handle, name: &str) {
    if matches!(name, "checked" | "name" | "type" | "form") {
        group(node);
    } else if name == "id" {
        owner_changes();
    }
}
pub(super) fn parsed(root: Handle) {
    for node in descendants(root) {
        refresh(node);
    }
    for node in descendants(root).into_iter().rev() {
        if with_tree(|tree| radio(tree, node)).unwrap_or(false) {
            group(node);
        }
    }
    owner_changes();
}
pub(super) fn inserted(ancestor: Handle) {
    inserted_many(&[ancestor]);
}
pub(super) fn inserted_many(ancestors: &[Handle]) {
    let mut rows = candidates()
        .into_iter()
        .filter_map(|node| {
            with_tree(|tree| {
                if !ancestors
                    .iter()
                    .any(|ancestor| within(tree, *ancestor, node))
                {
                    return None;
                }
                let owner = root(tree, node);
                path(tree, owner, node).map(|path| (owner.raw(), path, node))
            })
            .flatten()
        })
        .collect::<Vec<_>>();
    rows.sort_unstable_by(|left, right| (right.0, &right.1).cmp(&(left.0, &left.1)));
    for (_, _, node) in rows {
        group(node);
    }
}
pub(super) fn owner_changes() {
    let previous = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .map(|state| {
                state
                    .checked_radios
                    .iter()
                    .map(|(node, owner)| (*node, *owner))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let mut rows = previous
        .into_iter()
        .filter(|(node, owner)| closest_form(*node) != *owner)
        .filter_map(|(node, _)| {
            with_tree(|tree| {
                let owner = root(tree, node);
                path(tree, owner, node).map(|path| (owner.raw(), path, node))
            })
            .flatten()
        })
        .collect::<Vec<_>>();
    rows.sort_unstable_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
    for (_, _, node) in rows {
        group(node);
    }
}
pub(super) fn snapshot(node: Handle) -> Vec<(Handle, bool)> {
    let mut rows = peers(node)
        .into_iter()
        .filter(|peer| checked(*peer))
        .collect::<Vec<_>>();
    rows.push(node);
    rows.into_iter().map(|node| (node, checked(node))).collect()
}
pub(super) fn restore(rows: Vec<(Handle, bool)>) {
    with_tree_mut(|tree| {
        for &(node, state) in &rows {
            tree.set_form_property(node, FormProp::Checked, state)
        }
    });
    for (node, _) in rows {
        refresh(node);
    }
}
