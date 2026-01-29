use std::fmt;

use crate::{SupportRange, Version, deprecated, rules, unsupported};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeatureId {
    // Syntax
    SugarFoo,
    NewHeadingSyntax,

    // Semantics
    CounterBehaviorChange,
}

impl FeatureId {
    /// Stable machine-readable identifier (do not rename lightly).
    pub const fn key(self) -> &'static str {
        match self {
            FeatureId::SugarFoo => "sugar_foo",
            FeatureId::NewHeadingSyntax => "new_heading_syntax",
            FeatureId::CounterBehaviorChange => "counter_behavior_change",
        }
    }

    /// Human-facing label (can evolve).
    pub const fn label(self) -> &'static str {
        match self {
            FeatureId::SugarFoo => "SugarFoo syntax",
            FeatureId::NewHeadingSyntax => "New heading syntax",
            FeatureId::CounterBehaviorChange => "Counter behavior change",
        }
    }
}

/// Display = stable id (good for logs/JSON keys)
impl fmt::Display for FeatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

/// Centralized feature metadata (version gating + message).
#[derive(Clone, Copy, Debug)]
pub struct FeatureSpec {
    pub id: FeatureId,
    pub rules: Rules,
}

impl FeatureSpec {
    pub const fn new(id: FeatureId, rules: Rules) -> Self {
        Self { id, rules }
    }
}

pub const SUGAR_FOO: FeatureSpec = FeatureSpec::new(
    FeatureId::SugarFoo,
    rules!(unsupported!(to_incl 0, 14, max), deprecated!(0, 17, 0),),
);
pub const NEW_HEADING_SYNTAX: FeatureSpec = FeatureSpec::new(
    FeatureId::NewHeadingSyntax,
    rules!(unsupported!(to_incl 0, 14, max), deprecated!(0, 17, 0),),
);
pub const COUNTER_BEHAVIOR_CHANGE: FeatureSpec = FeatureSpec::new(
    FeatureId::CounterBehaviorChange,
    rules!(unsupported!(to_incl 0, 14, max), deprecated!(0, 17, 0),),
);

/// Resolve FeatureId → FeatureSpec.
/// (Centralization is the whole point.)
pub const fn spec(id: FeatureId) -> &'static FeatureSpec {
    match id {
        FeatureId::SugarFoo => &SUGAR_FOO,
        FeatureId::NewHeadingSyntax => &NEW_HEADING_SYNTAX,
        FeatureId::CounterBehaviorChange => &COUNTER_BEHAVIOR_CHANGE,
    }
}

impl FeatureId {
    pub const fn spec(self) -> &'static FeatureSpec {
        spec(self)
    }

    pub fn message(self, v: Version) -> String {
        self.spec().message(v)
    }

    pub fn supports(self, v: Version) -> bool {
        self.spec().supports(v)
    }

    pub fn deprecated(self, v: Version) -> bool {
        self.spec().deprecated(v)
    }
}

impl From<FeatureId> for &'static FeatureSpec {
    fn from(id: FeatureId) -> Self {
        id.spec()
    }
}

impl FeatureSpec {
    pub fn supports(&self, v: Version) -> bool {
        // Unsupported が当たったら false
        !self
            .rules
            .iter()
            .any(|r| r.kind == RuleKind::Unsupported && r.range.supports(v))
    }

    pub fn deprecated(&self, v: Version) -> bool {
        self.rules
            .iter()
            .any(|r| r.kind == RuleKind::Deprecated && r.range.supports(v))
    }

    pub fn message(&self, v: Version) -> String {
        if !self.supports(v) {
            return format!("{} is not supported in Typst {}.", self.id.label(), v);
        }
        if self.deprecated(v) {
            return format!("{} is deprecated in Typst {}.", self.id.label(), v);
        }
        format!("{} is supported in Typst {}.", self.id.label(), v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version;

    #[test]
    fn test_feature_id_keys() {
        assert_eq!(FeatureId::SugarFoo.key(), "sugar_foo");
        assert_eq!(FeatureId::NewHeadingSyntax.key(), "new_heading_syntax");
        assert_eq!(
            FeatureId::CounterBehaviorChange.key(),
            "counter_behavior_change"
        );
    }

    #[test]
    fn test_feature_id_labels() {
        assert_eq!(FeatureId::SugarFoo.label(), "SugarFoo syntax");
        assert_eq!(FeatureId::NewHeadingSyntax.label(), "New heading syntax");
        assert_eq!(
            FeatureId::CounterBehaviorChange.label(),
            "Counter behavior change"
        );
    }

    #[test]
    fn test_feature_support_gating() {
        let v_old = version!(0, 14, 0);
        let v_edge = version!(0, 14, max);
        let v_current = version!(0, 15, 0);
        let v_deprecated = version!(0, 17, 0);

        // SugarFoo
        assert!(!FeatureId::SugarFoo.supports(v_old));
        assert!(!FeatureId::SugarFoo.supports(v_edge));
        assert!(FeatureId::SugarFoo.supports(v_current));
        assert!(FeatureId::SugarFoo.supports(v_deprecated));
        assert!(FeatureId::SugarFoo.deprecated(v_deprecated));
    }

    #[test]
    fn test_feature_messages() {
        let v_old = version!(0, 14, 0);
        let v_supported = version!(0, 15, 0);
        let v_deprecated = version!(0, 17, 0);

        let id = FeatureId::SugarFoo;
        assert!(id.message(v_old).contains("is not supported"));
        assert!(id.message(v_supported).contains("is supported"));
        assert!(id.message(v_deprecated).contains("is deprecated"));
    }

    #[test]
    fn test_rules_iterator() {
        let spec = FeatureId::SugarFoo.spec();
        let mut count = 0;
        for rule in spec.rules.iter() {
            count += 1;
            let _ = rule.kind;
        }
        assert_eq!(count, 2);

        // test IntoIterator for &Rules
        let mut count2 = 0;
        for _ in &spec.rules {
            count2 += 1;
        }
        assert_eq!(count2, 2);
    }

    #[test]
    #[should_panic]
    fn test_empty_rules_panics() {
        // Empty rules set should panic
        Rules::new(&[]);
    }
}
