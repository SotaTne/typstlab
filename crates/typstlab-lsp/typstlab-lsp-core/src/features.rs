use std::fmt;

use crate::{SupportRange, Version, deprecated, rules, unsupported};
use core::slice::Iter;

pub trait Feature {
    const IGNORE: bool;
    const FEATURES: &'static [FeatureId];

    #[inline]
    fn ignore() -> bool {
        Self::IGNORE
    }

    #[inline]
    fn features() -> &'static [FeatureId] {
        if Self::IGNORE { &[] } else { Self::FEATURES }
    }

    #[inline]
    fn collect_unsupported(v: Version) -> Vec<FeatureId> {
        Self::features()
            .iter()
            .copied()
            .filter(|f| !f.supports(v))
            .collect()
    }

    #[inline]
    fn collect_deprecated(v: Version) -> Vec<FeatureId> {
        Self::features()
            .iter()
            .copied()
            .filter(|f| f.deprecated(v))
            .collect()
    }

    #[inline]
    fn has_unsupported(v: Version) -> bool {
        Self::features().iter().any(|f| !f.supports(v))
    }

    #[inline]
    fn has_deprecated(v: Version) -> bool {
        Self::features().iter().any(|f| f.deprecated(v))
    }
}

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

    // Testing / Integration (Stable variants for macro tests)
    #[cfg(any(test, debug_assertions))]
    TestV0_12_0Plus,
    #[cfg(any(test, debug_assertions))]
    TestV0_12_5ToV0_13_0,
    #[cfg(any(test, debug_assertions))]
    TestV0_13_2Plus,
}

impl FeatureId {
    /// Stable machine-readable identifier (do not rename lightly).
    pub const fn key(self) -> &'static str {
        match self {
            FeatureId::SugarFoo => "sugar_foo",
            FeatureId::NewHeadingSyntax => "new_heading_syntax",
            FeatureId::CounterBehaviorChange => "counter_behavior_change",
            #[cfg(any(test, debug_assertions))]
            FeatureId::TestV0_12_0Plus => "test_v0_12_0_plus",
            #[cfg(any(test, debug_assertions))]
            FeatureId::TestV0_12_5ToV0_13_0 => "test_v0_12_5_to_v0_13_0",
            #[cfg(any(test, debug_assertions))]
            FeatureId::TestV0_13_2Plus => "test_v0_13_2_plus",
        }
    }

    /// Human-facing label (can evolve).
    pub const fn label(self) -> &'static str {
        match self {
            FeatureId::SugarFoo => "SugarFoo syntax",
            FeatureId::NewHeadingSyntax => "New heading syntax",
            FeatureId::CounterBehaviorChange => "Counter behavior change",
            #[cfg(any(test, debug_assertions))]
            FeatureId::TestV0_12_0Plus => "Test Feature (0.12.0+)",
            #[cfg(any(test, debug_assertions))]
            FeatureId::TestV0_12_5ToV0_13_0 => "Test Feature (0.12.5 - 0.13.0)",
            #[cfg(any(test, debug_assertions))]
            FeatureId::TestV0_13_2Plus => "Test Feature (0.13.2+)",
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
#[cfg(any(test, debug_assertions))]
pub const TEST_V0_12_0_PLUS: FeatureSpec = FeatureSpec::new(
    FeatureId::TestV0_12_0Plus,
    rules!(unsupported!(to_incl 0, 11, max),),
);
#[cfg(any(test, debug_assertions))]
pub const TEST_V0_12_5_TO_V0_13_0: FeatureSpec = FeatureSpec::new(
    FeatureId::TestV0_12_5ToV0_13_0,
    rules!(unsupported!(to_incl 0, 12, 4), deprecated!(0, 13, 0),),
);
#[cfg(any(test, debug_assertions))]
pub const TEST_V0_13_2_PLUS: FeatureSpec = FeatureSpec::new(
    FeatureId::TestV0_13_2Plus,
    rules!(unsupported!(to_incl 0, 13, 1),),
);

/// Resolve FeatureId → FeatureSpec.
/// (Centralization is the whole point.)
pub const fn spec(id: FeatureId) -> &'static FeatureSpec {
    match id {
        FeatureId::SugarFoo => &SUGAR_FOO,
        FeatureId::NewHeadingSyntax => &NEW_HEADING_SYNTAX,
        FeatureId::CounterBehaviorChange => &COUNTER_BEHAVIOR_CHANGE,
        #[cfg(any(test, debug_assertions))]
        FeatureId::TestV0_12_0Plus => &TEST_V0_12_0_PLUS,
        #[cfg(any(test, debug_assertions))]
        FeatureId::TestV0_12_5ToV0_13_0 => &TEST_V0_12_5_TO_V0_13_0,
        #[cfg(any(test, debug_assertions))]
        FeatureId::TestV0_13_2Plus => &TEST_V0_13_2_PLUS,
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
        assert_eq!(FeatureId::TestV0_12_0Plus.key(), "test_v0_12_0_plus");
        assert_eq!(
            FeatureId::TestV0_12_5ToV0_13_0.key(),
            "test_v0_12_5_to_v0_13_0"
        );
        assert_eq!(FeatureId::TestV0_13_2Plus.key(), "test_v0_13_2_plus");
    }

    #[test]
    fn test_feature_id_labels() {
        assert_eq!(FeatureId::TestV0_12_0Plus.label(), "Test Feature (0.12.0+)");
        assert_eq!(
            FeatureId::TestV0_12_5ToV0_13_0.label(),
            "Test Feature (0.12.5 - 0.13.0)"
        );
    }

    #[test]
    fn test_feature_support_gating() {
        let v_old = version!(0, 11, max);
        let v_12_0 = version!(0, 12, 0);
        let v_12_4 = version!(0, 12, 4);
        let v_12_5 = version!(0, 12, 5);
        let v_13_0 = version!(0, 13, 0);
        let v_13_2 = version!(0, 13, 2);

        // TestV0_12_0Plus: 0.12.0+
        assert!(!FeatureId::TestV0_12_0Plus.supports(v_old));
        assert!(FeatureId::TestV0_12_0Plus.supports(v_12_0));

        // TestV0_12_5ToV0_13_0: 0.12.5+, Deprecated 0.13.0+
        assert!(!FeatureId::TestV0_12_5ToV0_13_0.supports(v_12_4));
        assert!(FeatureId::TestV0_12_5ToV0_13_0.supports(v_12_5));
        assert!(FeatureId::TestV0_12_5ToV0_13_0.deprecated(v_13_0));

        // TestV0_13_2Plus: 0.13.2+
        assert!(!FeatureId::TestV0_13_2Plus.supports(v_13_0));
        assert!(FeatureId::TestV0_13_2Plus.supports(v_13_2));
    }

    #[test]
    fn test_feature_messages() {
        let v_old = version!(0, 11, 0);
        let v_supported = version!(0, 12, 0);
        let v_deprecated = version!(0, 13, 0);

        let id = FeatureId::TestV0_12_5ToV0_13_0;
        assert!(id.message(v_old).contains("is not supported"));
        assert!(id.message(v_supported).contains("is not supported")); // 0.12.5 からなので
        assert!(id.message(version!(0, 12, 5)).contains("is supported"));
        assert!(id.message(v_deprecated).contains("is deprecated"));
    }

    #[test]
    fn test_rules_iterator() {
        let spec = FeatureId::TestV0_12_5ToV0_13_0.spec();
        let mut count = 0;
        for rule in spec.rules.iter() {
            count += 1;
            let _ = rule.kind;
        }
        assert_eq!(count, 2); // Unsupported and Deprecated

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
