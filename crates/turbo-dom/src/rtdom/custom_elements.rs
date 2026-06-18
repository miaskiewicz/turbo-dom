//! Pure-Rust port of the customElements registry (`makeCustomElements` in
//! `src/runtime/stubs.mjs`). NO wasm/napi, no `Tree` dependency — a standalone
//! name→definition map.
//!
//! The JS version maps a tag name to a JS *class constructor* and exposes
//! `define`/`get`/`getName`/`whenDefined`/`upgrade`. A JS class constructor is not
//! representable in Rust, so this port stores an opaque DEFINITION RECORD
//! ([`CustomElementDefinition`]) instead — the plain data a definition carries
//! (name, `is=`-extends target, observed attributes). The embedder supplies and
//! owns whatever element factory / upgrade machinery it wants, keyed by name.
//!
//! Explicitly the EMBEDDER's concern (intentionally not ported here):
//!   * the JS-class constructor itself (no Rust analogue),
//!   * `whenDefined` (a `Promise` — an async/runtime concern; poll [`is_defined`]
//!     or look the name up after definition),
//!   * `upgrade()` (a no-op in the JS stub anyway).
//!
//! Re-define behavior: the JS stub THROWS on a duplicate name. For a panic-free,
//! testable Rust port, [`CustomElementRegistry::define`] returns
//! `Result<(), String>` and rejects a duplicate with `Err` (mirroring the JS
//! throw) — the registry is never mutated on the error path.

/// The plain data a custom-element definition carries, minus the (non-portable)
/// JS class constructor. The embedder associates its own factory with `name`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomElementDefinition {
    /// The custom element tag name (e.g. `"my-widget"`).
    pub name: String,
    /// For customized built-ins (`is=`), the built-in tag being extended.
    pub extends: Option<String>,
    /// Attribute names the element observes for change callbacks.
    pub observed_attributes: Vec<String>,
}

impl CustomElementDefinition {
    /// Convenience constructor for an autonomous custom element (no `extends`,
    /// no observed attributes).
    pub fn new(name: &str) -> Self {
        CustomElementDefinition {
            name: name.to_string(),
            extends: None,
            observed_attributes: Vec::new(),
        }
    }
}

/// A portable customElements registry: name → opaque definition record.
///
/// Mirrors the portable parts of the JS `makeCustomElements` stub
/// (`define`/`get`, plus `is_defined`/`defined_names`). The JS-class constructor
/// and the `whenDefined` Promise are the embedder's concern (see module docs).
#[derive(Debug, Clone, Default)]
pub struct CustomElementRegistry {
    defs: std::collections::HashMap<String, CustomElementDefinition>,
}

impl CustomElementRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        CustomElementRegistry {
            defs: std::collections::HashMap::new(),
        }
    }

    /// Define a custom element. Validates that `name` is a valid custom-element
    /// name (per spec: must contain a `-` and start with a lowercase ASCII
    /// letter) and that it is not already defined.
    ///
    /// Returns `Err` (registry unchanged) when the name is invalid or already
    /// defined — mirroring the JS stub's throw, but panic-free.
    pub fn define(&mut self, name: &str, def: CustomElementDefinition) -> Result<(), String> {
        if !is_valid_name(name) {
            return Err(format!("'{name}' is not a valid custom element name"));
        }
        if self.defs.contains_key(name) {
            return Err(format!("'{name}' already defined"));
        }
        self.defs.insert(name.to_string(), def);
        Ok(())
    }

    /// Look up the definition record for a name. `None` if undefined.
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition> {
        self.defs.get(name)
    }

    /// Whether a name has been defined.
    pub fn is_defined(&self, name: &str) -> bool {
        self.defs.contains_key(name)
    }

    /// All defined names (unordered — backed by a `HashMap`).
    pub fn defined_names(&self) -> Vec<&str> {
        self.defs.keys().map(String::as_str).collect()
    }
}

/// Valid-custom-element-name check (the portable subset of the HTML spec rule
/// the JS stub relies on the platform for): a non-empty name that starts with a
/// lowercase ASCII letter and contains at least one `-`.
fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    name.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_new_defaults() {
        let d = CustomElementDefinition::new("my-el");
        assert_eq!(d.name, "my-el");
        assert_eq!(d.extends, None);
        assert!(d.observed_attributes.is_empty());
        // Default impl path.
        let def: CustomElementDefinition = Default::default();
        assert_eq!(def.name, "");
    }

    #[test]
    fn define_get_is_defined() {
        let mut reg = CustomElementRegistry::new();
        let mut def = CustomElementDefinition::new("x-card");
        def.extends = Some("div".to_string());
        def.observed_attributes = vec!["open".to_string()];

        assert!(!reg.is_defined("x-card"));
        assert!(reg.get("x-card").is_none());

        assert!(reg.define("x-card", def.clone()).is_ok());

        assert!(reg.is_defined("x-card"));
        let got = reg.get("x-card").unwrap();
        assert_eq!(got.name, "x-card");
        assert_eq!(got.extends.as_deref(), Some("div"));
        assert_eq!(got.observed_attributes, vec!["open".to_string()]);
    }

    #[test]
    fn get_unknown_is_none() {
        let reg = CustomElementRegistry::new();
        assert!(reg.get("nope-nope").is_none());
        assert!(!reg.is_defined("nope-nope"));
    }

    #[test]
    fn invalid_name_no_hyphen_errs() {
        let mut reg = CustomElementRegistry::new();
        let err = reg
            .define("nohyphen", CustomElementDefinition::new("nohyphen"))
            .unwrap_err();
        assert!(err.contains("not a valid"));
        assert!(!reg.is_defined("nohyphen"));
    }

    #[test]
    fn invalid_name_bad_start_errs() {
        let mut reg = CustomElementRegistry::new();
        // Empty name (no first char).
        assert!(reg.define("", CustomElementDefinition::new("")).is_err());
        // Starts with uppercase.
        assert!(reg
            .define("My-el", CustomElementDefinition::new("My-el"))
            .is_err());
        // Starts with a digit.
        assert!(reg
            .define("1-el", CustomElementDefinition::new("1-el"))
            .is_err());
    }

    #[test]
    fn redefine_errs_and_keeps_original() {
        let mut reg = CustomElementRegistry::new();
        let mut first = CustomElementDefinition::new("dup-el");
        first.observed_attributes = vec!["a".to_string()];
        assert!(reg.define("dup-el", first).is_ok());

        let mut second = CustomElementDefinition::new("dup-el");
        second.observed_attributes = vec!["b".to_string()];
        let err = reg.define("dup-el", second).unwrap_err();
        assert!(err.contains("already defined"));

        // Original definition is preserved (registry unchanged on error).
        assert_eq!(
            reg.get("dup-el").unwrap().observed_attributes,
            vec!["a".to_string()]
        );
    }

    #[test]
    fn defined_names_lists_all() {
        let mut reg = CustomElementRegistry::new();
        reg.define("a-one", CustomElementDefinition::new("a-one"))
            .unwrap();
        reg.define("b-two", CustomElementDefinition::new("b-two"))
            .unwrap();
        let mut names = reg.defined_names();
        names.sort_unstable();
        assert_eq!(names, vec!["a-one", "b-two"]);
    }
}
