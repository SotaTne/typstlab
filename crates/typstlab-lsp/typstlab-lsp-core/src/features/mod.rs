pub mod id;
pub mod rules;
pub mod spec;
pub mod table;
pub mod traits;

pub use id::FeatureId;
pub use rules::{FeatureRule, RuleKind, Rules};
pub use spec::FeatureSpec;
pub use table::*;
pub use traits::Feature;

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
