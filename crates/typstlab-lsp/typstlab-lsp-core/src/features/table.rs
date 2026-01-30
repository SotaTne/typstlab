use crate::FeatureId;
use crate::FeatureSpec;
use crate::deprecated;
use crate::rules;
use crate::unsupported;

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
