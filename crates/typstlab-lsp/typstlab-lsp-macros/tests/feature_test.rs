use typstlab_lsp_core::{Feature, FeatureId};
use typstlab_lsp_macros::Feature as DeriveFeature;

#[derive(DeriveFeature)]
#[feat(TestV0_12_0Plus)]
struct SingleFeature;

#[test]
fn test_single_feature() {
    assert!(!SingleFeature::IGNORE);
    assert_eq!(SingleFeature::features().len(), 1);
    assert_eq!(SingleFeature::features()[0], FeatureId::TestV0_12_0Plus);
}

#[derive(DeriveFeature)]
#[feat(TestV0_12_0Plus, FeatureId::TestV0_12_5ToV0_13_0, TestV0_13_2Plus)]
struct MultiFeature;

#[test]
fn test_multi_feature() {
    assert!(!MultiFeature::IGNORE);
    let feats = MultiFeature::features();
    assert_eq!(feats.len(), 3);
    assert!(feats.contains(&FeatureId::TestV0_12_0Plus));
    assert!(feats.contains(&FeatureId::TestV0_12_5ToV0_13_0));
    assert!(feats.contains(&FeatureId::TestV0_13_2Plus));
}

#[derive(DeriveFeature)]
#[feat(ignore)]
struct IgnoredNode;

#[test]
fn test_ignored_feature() {
    assert!(IgnoredNode::IGNORE);
    assert!(IgnoredNode::features().is_empty());
}

#[derive(DeriveFeature)]
#[feat(TestV0_12_0Plus)]
enum FeatureEnum {
    #[allow(dead_code)]
    Variant,
}

#[test]
fn test_enum_feature() {
    assert!(!FeatureEnum::IGNORE);
    assert_eq!(FeatureEnum::FEATURES[0], FeatureId::TestV0_12_0Plus);
}
