//! Form-state pseudo-classes for the selector APIs (querySelector[All], matches, closest).
//!
//! rtdom's matcher only knows attribute-level approximations of these states (and lacks
//! `:indeterminate`, `:default`, `:placeholder-shown`, the range and validity classes). Their
//! truth lives in the binding: IDL-only indeterminate flags, checkedness and selectedness,
//! constraint validation and the user-validity flag a submission attempt sets. As
//! `:popover-open` and `:modal` do, each occurrence is replaced by structural selectors for the
//! elements of the queried root that match, classified by the private helper captured from
//! form_pseudo.js. `:read-only` is the complement of `:read-write`. The same transform answers
//! `:scope`, which rtdom does not know: the queried element itself, the document element for a
//! document, and nothing for a fragment.
use super::*;

struct Classifier(v8::Weak<v8::Function>);

const NAMES: [&str; 16] = [
    "checked",
    "indeterminate",
    "default",
    "placeholder-shown",
    "read-only",
    "read-write",
    "in-range",
    "out-of-range",
    "valid",
    "invalid",
    "user-valid",
    "user-invalid",
    "enabled",
    "disabled",
    "required",
    "optional",
];

fn pseudo_name(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("scope") {
        return Some("scope");
    }
    NAMES
        .iter()
        .copied()
        .find(|known| known.eq_ignore_ascii_case(name))
}

fn scope_selector(tree: &Tree, anchor: Handle) -> String {
    match tree.node_type_id(anchor) {
        1 => format!(":is({})", html_popover::selector_path(tree, anchor)),
        9 => ":root".to_owned(),
        _ => ":not(*)".to_owned(),
    }
}

/// Byte ranges of `:name` occurrences, inside functional arguments too.
fn pseudo_ranges<'i>(
    input: &mut cssparser::Parser<'i, '_>,
    ranges: &mut Vec<(usize, usize, &'static str)>,
) -> Result<(), cssparser::ParseError<'i, ()>> {
    use cssparser::Token;
    let mut colon = None;
    let mut double_colon = false;
    while !input.is_exhausted() {
        let position = input.position().byte_index();
        let token = input.next_including_whitespace_and_comments()?.clone();
        match token {
            Token::Colon => {
                double_colon = colon.is_some();
                colon = if double_colon { None } else { Some(position) };
            }
            Token::Ident(ref name) => {
                if let (false, Some(start), Some(pseudo)) = (double_colon, colon, pseudo_name(name))
                {
                    ranges.push((start, input.position().byte_index(), pseudo));
                }
                colon = None;
                double_colon = false;
            }
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                colon = None;
                double_colon = false;
                input.parse_nested_block(|nested| pseudo_ranges(nested, ranges))?;
            }
            Token::Comment(_) => (),
            _ => {
                colon = None;
                double_colon = false;
            }
        }
    }
    Ok(())
}

/// Every element of `root`'s tree (the root itself when it is one), in tree order.
fn tree_elements(tree: &Tree, root: Handle) -> Vec<Handle> {
    let mut elements = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if tree.node_type_id(node) == 1 {
            elements.push(node);
        }
        stack.extend(tree.children(node).into_iter().rev());
    }
    elements
}

/// The matching elements of `root` for each requested pseudo-class, or `None` when this realm
/// has no classifier (the selector then keeps rtdom's own matching).
fn classify(
    scope: &mut v8::PinScope,
    root: Handle,
    names: &[&'static str],
) -> Option<Vec<Vec<Handle>>> {
    let classifier = scope
        .get_current_context()
        .get_slot::<Classifier>()
        .and_then(|slot| slot.0.to_local(scope))?;
    let handles = with_tree(|tree| tree_elements(tree, root))?;
    let elements: Vec<v8::Local<v8::Value>> = handles
        .iter()
        .map(|&handle| wrap(scope, handle).into())
        .collect();
    let elements = v8::Array::new_with_elements(scope, &elements);
    let requested: Vec<v8::Local<v8::Value>> = names
        .iter()
        .map(|name| v8::String::new(scope, name).unwrap().into())
        .collect();
    let requested = v8::Array::new_with_elements(scope, &requested);
    let receiver = v8::undefined(scope).into();
    let result = classifier.call(scope, receiver, &[elements.into(), requested.into()])?;
    let result = v8::Local::<v8::Array>::try_from(result).ok()?;
    let mut sets = Vec::with_capacity(names.len());
    for index in 0..names.len() as u32 {
        let matched = result
            .get_index(scope, index)
            .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())?;
        let mut set = Vec::with_capacity(matched.length() as usize);
        for at in 0..matched.length() {
            let element = matched
                .get_index(scope, at)
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())?;
            set.push(handle_of(scope, element)?);
        }
        sets.push(set);
    }
    Some(sets)
}

pub(super) fn selector(scope: &mut v8::PinScope, anchor: Handle, source: &str) -> Option<String> {
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    if pseudo_ranges(&mut parser, &mut ranges).is_err() || ranges.is_empty() {
        return Some(source.to_owned());
    }
    let mut names: Vec<&'static str> = Vec::new();
    for &(_, _, name) in ranges.iter().filter(|range| range.2 != "scope") {
        // `:read-only` is answered by its complement.
        let name = if name == "read-only" {
            "read-write"
        } else {
            name
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let (root, scoped) = with_tree(|tree| {
        (
            html_popover::selector_root(tree, anchor),
            scope_selector(tree, anchor),
        )
    })?;
    let sets = if names.is_empty() {
        Vec::new()
    } else {
        match classify(scope, root, &names) {
            Some(sets) => sets,
            None => return Some(source.to_owned()),
        }
    };
    let replacements: Vec<(&'static str, String)> = with_tree(|tree| {
        names
            .iter()
            .zip(&sets)
            .map(|(&name, set)| {
                let paths: Vec<String> = set
                    .iter()
                    .map(|&node| html_popover::selector_path(tree, node))
                    .collect();
                (name, paths.join(","))
            })
            .collect()
    })?;
    let replacement = |name: &str| -> String {
        if name == "scope" {
            return scoped.clone();
        }
        let lookup = if name == "read-only" {
            "read-write"
        } else {
            name
        };
        let paths = &replacements
            .iter()
            .find(|(known, _)| *known == lookup)
            .unwrap()
            .1;
        match (name == "read-only", paths.is_empty()) {
            (false, true) => ":not(*)".to_owned(),
            (false, false) => format!(":is({paths})"),
            (true, true) => ":is(*)".to_owned(),
            (true, false) => format!(":not(:is({paths}))"),
        }
    };
    let mut result = String::new();
    let mut start = 0;
    for (from, to, name) in ranges {
        result.push_str(&source[start..from]);
        result.push_str(&replacement(name));
        start = to;
    }
    result.push_str(&source[start..]);
    Some(result)
}

pub(super) fn install(scope: &mut v8::PinScope) {
    run_js(scope, include_str!("form_pseudo.js"));
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__captureFormPseudo").unwrap();
    if let Some(callback) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(callback) = dom_ownership::helper_function(scope, callback) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(Classifier(callback)));
        }
    }
    global.delete(scope, key.into());
}
