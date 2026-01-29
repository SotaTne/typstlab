pub mod close;
pub mod error;
pub mod features;
pub mod macros;
pub mod version;

pub use close::Close;
pub use error::ErrorCode;
pub use features::{Feature, FeatureId, FeatureRule, FeatureSpec, RuleKind, Rules};
pub use version::{SupportRange, Version};

/// Attach version-gating metadata to AST nodes / semantic nodes / etc.
///
/// The macros crate will generate impls of this trait.
pub trait VersionGated {
    /// Which feature this node depends on (if any).
    fn feature_id() -> Option<FeatureId> {
        None
    }

    /// Optional direct support range (useful for ad-hoc gating).
    /// If `feature_id()` is Some, you can resolve the range via `features::spec(feature)`.
    fn support_range() -> Option<SupportRange> {
        None
    }
}
