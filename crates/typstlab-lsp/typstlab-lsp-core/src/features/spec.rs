use crate::FeatureId;
use crate::RuleKind;
use crate::Rules;
use crate::Version;

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
