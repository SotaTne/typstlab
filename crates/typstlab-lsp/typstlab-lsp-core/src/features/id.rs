use std::fmt;

use crate::{FeatureSpec, Version, features::spec};

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
