//! Pure cascade ordering shared by stylesheet adapters; no selector or value policy.
use std::cmp::Reverse;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CascadeOrigin {
    UserAgent,
    User,
    Author,
}

/// Values leave the CSS animation and transition levels in their standard slots.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CascadeLevel {
    UserAgent,
    User,
    Author,
    Animation,
    AuthorImportant,
    UserImportant,
    UserAgentImportant,
    Transition,
}
impl CascadeOrigin {
    #[must_use]
    pub fn level(self, important: bool) -> CascadeLevel {
        match (self, important) {
            (Self::UserAgent, false) => CascadeLevel::UserAgent,
            (Self::User, false) => CascadeLevel::User,
            (Self::Author, false) => CascadeLevel::Author,
            (Self::Author, true) => CascadeLevel::AuthorImportant,
            (Self::User, true) => CascadeLevel::UserImportant,
            (Self::UserAgent, true) => CascadeLevel::UserAgentImportant,
        }
    }
    #[must_use]
    pub fn normal_order(self) -> u8 {
        match self {
            Self::UserAgent => 0,
            Self::User => 1,
            Self::Author => 2,
        }
    }
}

#[must_use]
pub fn encapsulation_rank(depth: u32, important: bool) -> u32 {
    if important {
        depth
    } else {
        u32::MAX - depth
    }
}

/// Compare lexicographically: the higher rank wins. The caller has already selected
/// an eligible scope root; smaller proximity then wins before source order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CascadeRank<'a> {
    pub level: CascadeLevel,
    pub encapsulation: u32,
    pub inline: bool,
    pub layer: &'a [usize],
    pub specificity: (u32, u32, u32),
    pub proximity: Reverse<usize>,
    pub order: (usize, usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origins_and_effect_levels_follow_css_not_boolean_importance_alone() {
        let ua = CascadeOrigin::UserAgent;
        let user = CascadeOrigin::User;
        let author = CascadeOrigin::Author;
        assert!(ua.level(false) < user.level(false));
        assert!(user.level(false) < author.level(false));
        assert!(author.level(false) < CascadeLevel::Animation);
        assert!(CascadeLevel::Animation < author.level(true));
        assert!(author.level(true) < user.level(true));
        assert!(user.level(true) < ua.level(true));
        assert!(ua.level(true) < CascadeLevel::Transition);
        assert_eq!(
            [
                ua.normal_order(),
                user.normal_order(),
                author.normal_order()
            ],
            [0, 1, 2]
        );
    }
    #[test]
    fn outer_normal_and_inner_important_encapsulation_reverse() {
        assert!(encapsulation_rank(0, false) > encapsulation_rank(1, false));
        assert!(encapsulation_rank(0, true) < encapsulation_rank(1, true));
        let base = CascadeRank {
            level: CascadeLevel::Author,
            encapsulation: 0,
            inline: false,
            layer: &[],
            specificity: (0, 1, 0),
            proximity: Reverse(2),
            order: (0, 0),
        };
        let distant = CascadeRank {
            proximity: Reverse(3),
            order: (100, 100),
            ..base
        };
        assert!(base > distant);
    }
}
