//! Layer identity/order, independent of selector matching and the DOM adapter.
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerError {
    LayerCount,
    InvalidParent,
    EmptyName,
}

#[derive(Default)]
struct Layer {
    parent: Option<usize>,
    index: usize,
    children: FxHashMap<String, usize>,
    child_count: usize,
}

/// The root is a registry node, not an author-visible layer. Anonymous identities
/// never enter a named lookup, so separate anonymous blocks cannot merge.
pub struct LayerRegistry {
    layers: Vec<Layer>,
    maximum: usize,
}

impl LayerRegistry {
    #[must_use]
    pub fn new(maximum: usize) -> Self {
        Self {
            layers: vec![Layer::default()],
            maximum,
        }
    }

    pub(super) fn layer_count(&self) -> usize {
        self.layers.len() - 1
    }

    pub(super) fn set_remaining(&mut self, remaining: usize) {
        self.maximum = self.layer_count().saturating_add(remaining);
    }

    fn add(&mut self, parent: usize) -> Result<usize, LayerError> {
        if self.layers.len() > self.maximum {
            return Err(LayerError::LayerCount);
        }
        let node = self
            .layers
            .get_mut(parent)
            .ok_or(LayerError::InvalidParent)?;
        let index = node.child_count;
        node.child_count += 1;
        let id = self.layers.len();
        self.layers.push(Layer {
            parent: Some(parent),
            index,
            ..Layer::default()
        });
        Ok(id)
    }

    /// `parent=None` is the document's outer layer context. Names are already
    /// decoded by the canonical layer-name grammar, never split on authored dots.
    pub fn named(&mut self, parent: Option<usize>, names: &[String]) -> Result<usize, LayerError> {
        if names.is_empty() {
            return Err(LayerError::EmptyName);
        }
        let mut parent = parent.unwrap_or(0);
        for name in names {
            let layer = self.layers.get(parent).ok_or(LayerError::InvalidParent)?;
            parent = if let Some(id) = layer.children.get(name).copied() {
                id
            } else {
                let id = self.add(parent)?;
                self.layers[parent].children.insert(name.clone(), id);
                id
            };
        }
        Ok(parent)
    }

    pub fn anonymous(&mut self, parent: Option<usize>) -> Result<usize, LayerError> {
        self.add(parent.unwrap_or(0))
    }

    /// Normal declarations in an outer layer follow its nested layers. Important
    /// declarations reverse that same hierarchy, including the implicit outer slot.
    pub fn rank(&self, layer: Option<usize>, important: bool) -> Result<Vec<usize>, LayerError> {
        let mut path = Vec::new();
        let mut current = layer.unwrap_or(0);
        while current != 0 {
            let node = self.layers.get(current).ok_or(LayerError::InvalidParent)?;
            path.push(node.index);
            current = node.parent.ok_or(LayerError::InvalidParent)?;
        }
        path.reverse();
        path.push(usize::MAX);
        if important {
            for index in &mut path {
                *index = usize::MAX - *index;
            }
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn name(value: &str) -> Vec<String> {
        vec![value.to_owned()]
    }
    #[test]
    fn named_reuse_and_anonymous_blocks_have_distinct_stable_identity() {
        let mut registry = LayerRegistry::new(8);
        let early = registry.named(None, &name("early")).unwrap();
        let anonymous = registry.anonymous(None).unwrap();
        let late = registry.named(None, &name("late")).unwrap();
        assert_eq!(registry.named(None, &name("early")).unwrap(), early);
        assert_ne!(early, anonymous);
        assert!(
            registry.rank(Some(early), false).unwrap()
                < registry.rank(Some(anonymous), false).unwrap()
        );
        assert!(
            registry.rank(Some(anonymous), false).unwrap()
                < registry.rank(Some(late), false).unwrap()
        );
        assert!(registry.rank(Some(late), false).unwrap() < registry.rank(None, false).unwrap());
        assert!(
            registry.rank(Some(early), true).unwrap() > registry.rank(Some(late), true).unwrap()
        );
        assert!(registry.rank(Some(late), true).unwrap() > registry.rank(None, true).unwrap());
    }
    #[test]
    fn nested_layers_and_escaped_dot_identifiers_are_not_flat_names() {
        let mut registry = LayerRegistry::new(8);
        let outer = registry.named(None, &name("app")).unwrap();
        let nested = registry.named(Some(outer), &name("reset")).unwrap();
        assert_eq!(
            registry
                .named(None, &["app".to_owned(), "reset".to_owned()])
                .unwrap(),
            nested
        );
        let dotted = registry.named(None, &name("app.reset")).unwrap();
        assert_ne!(nested, dotted);
        assert!(
            registry.rank(Some(nested), false).unwrap()
                < registry.rank(Some(outer), false).unwrap()
        );
        assert!(
            registry.rank(Some(nested), true).unwrap() > registry.rank(Some(outer), true).unwrap()
        );
    }
    #[test]
    fn explicit_bounds_and_unknown_contexts_never_fall_back_to_unlayered() {
        let mut registry = LayerRegistry::new(0);
        assert_eq!(registry.anonymous(None), Err(LayerError::LayerCount));
        assert_eq!(registry.named(None, &[]), Err(LayerError::EmptyName));
        assert_eq!(
            registry.named(Some(100), &name("x")),
            Err(LayerError::InvalidParent)
        );
        assert_eq!(
            registry.rank(Some(100), false),
            Err(LayerError::InvalidParent)
        );
        let mut registry = LayerRegistry::new(1);
        assert_eq!(
            registry.anonymous(Some(100)),
            Err(LayerError::InvalidParent)
        );
        registry.anonymous(None).unwrap();
        assert_eq!(
            registry.named(None, &name("x")),
            Err(LayerError::LayerCount)
        );
    }
}
