use crate::SupportRange;
use core::slice::Iter;

#[derive(Clone, Copy, Debug)]
pub struct Rules(pub &'static [FeatureRule]);

impl Rules {
    pub const fn new(rules: &'static [FeatureRule]) -> Self {
        assert!(!rules.is_empty());
        Self(rules)
    }

    pub fn iter(&self) -> Iter<'static, FeatureRule> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Rules {
    type Item = &'static FeatureRule;
    type IntoIter = Iter<'static, FeatureRule>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuleKind {
    Unsupported,
    Deprecated,
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureRule {
    pub range: SupportRange,
    pub kind: RuleKind,
}
