//! Shared transient selector answer domains, with no DOM handles or retained world cache.
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Debug)]
pub struct AnswerDomain(BTreeSet<u32>);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactError {
    DuplicateNode,
    OutsideDomain,
    MissingAnswer,
}
impl AnswerDomain {
    pub fn new(nodes: Vec<u32>) -> Result<Arc<Self>, FactError> {
        let count = nodes.len();
        let nodes: BTreeSet<_> = nodes.into_iter().collect();
        if nodes.len() != count {
            return Err(FactError::DuplicateNode);
        }
        Ok(Arc::new(Self(nodes)))
    }
}
pub struct MechanicalAnswer {
    domain: Arc<AnswerDomain>,
    matches: BTreeSet<u32>,
}
impl MechanicalAnswer {
    /// Validate membership once per bulk answer, rather than rescanning a shared
    /// domain for every distinct target's style read.
    pub fn new(domain: Arc<AnswerDomain>, matches: Vec<u32>) -> Result<Self, FactError> {
        let count = matches.len();
        let matches: BTreeSet<_> = matches.into_iter().collect();
        if matches.len() != count {
            return Err(FactError::DuplicateNode);
        }
        if matches.iter().any(|node| !domain.0.contains(node)) {
            return Err(FactError::OutsideDomain);
        }
        Ok(Self { domain, matches })
    }
    pub fn matches(&self, node: u32) -> Result<bool, FactError> {
        if !self.domain.0.contains(&node) {
            return Err(FactError::MissingAnswer);
        }
        Ok(self.matches.contains(&node))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_negatives_require_exhaustive_domain_membership() {
        let domain = AnswerDomain::new(vec![2, 1, 3]).unwrap();
        let first = MechanicalAnswer::new(domain.clone(), vec![2]).unwrap();
        let second = MechanicalAnswer::new(domain, vec![3]).unwrap();
        assert_eq!(first.matches(2), Ok(true));
        assert_eq!(first.matches(1), Ok(false));
        assert_eq!(second.matches(3), Ok(true));
        assert_eq!(first.matches(4), Err(FactError::MissingAnswer));
        assert_eq!(
            AnswerDomain::new(vec![1, 1]).unwrap_err(),
            FactError::DuplicateNode
        );
        let domain = AnswerDomain::new(vec![1]).unwrap();
        assert!(matches!(
            MechanicalAnswer::new(domain.clone(), vec![2]),
            Err(FactError::OutsideDomain)
        ));
        assert!(matches!(
            MechanicalAnswer::new(domain, vec![1, 1]),
            Err(FactError::DuplicateNode)
        ));
    }
}
