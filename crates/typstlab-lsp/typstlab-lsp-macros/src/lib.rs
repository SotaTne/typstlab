mod close;
mod close_cfg;
mod feature;
mod feature_cfg;
mod utils;

use proc_macro::TokenStream;

/// Derives the `Close` trait for a struct or enum.
///
/// This macro automatically generates implementations for `close()` and `close_and_shrink()`.
/// It recursively calls `close()` on all fields (or `close_and_shrink()` if specified).
///
/// # Attributes
///
/// - `#[close(skip)]`: Skips the field during cleanup.
/// - `#[close(shrink)]`: On a field, it forces `close_and_shrink()` to be called even during a regular `close()`.
///   On a type (struct/enum), it makes `shrink` the default mode for all its fields.
///
/// # Example
///
/// ```
/// use typstlab_lsp_core::Close;
/// use typstlab_lsp_macros::Close as DeriveClose;
///
/// #[derive(DeriveClose, Default)]
/// struct Resource {
///     #[close(shrink)]
///     buffer: Vec<u8>,
///     #[close(skip)]
///     id: u32,
///     metadata: String,
/// }
///
/// let mut res = Resource {
///     buffer: vec![1, 2, 3],
///     id: 100,
///     metadata: "hello".to_string(),
/// };
///
/// res.close();
///
/// assert!(res.buffer.is_empty());
/// assert_eq!(res.buffer.capacity(), 0); // shrunk because of #[close(shrink)]
/// assert_eq!(res.id, 100);             // skipped
/// assert!(res.metadata.is_empty());    // normal close
/// ```
#[proc_macro_derive(Close, attributes(close))]
pub fn derive_close(input: TokenStream) -> TokenStream {
    close::derive_close(input)
}

/// Derives the `Feature` trait for a struct or enum.
///
/// This macro generates the `IGNORE` and `FEATURES` constants required by the `Feature` trait.
///
/// # Attributes
///
/// - `#[feat(ignore)]`: Excludes the type from feature checking (`IGNORE = true`).
/// - `#[feat(ID1, ID2, ...)]`: Specifies the `FeatureId`s associated with this type.
///   IDs can be specified as simple names (`SugarFoo`) or paths (`FeatureId::SugarFoo`).
///
/// # Example
///
/// ```
/// use typstlab_lsp_core::{Feature, FeatureId};
/// use typstlab_lsp_macros::Feature as DeriveFeature;
///
/// #[derive(DeriveFeature)]
/// #[feat(TestV0_12_0Plus, TestV0_12_5ToV0_13_0)]
/// struct MultiFeatureNode;
///
/// assert!(!MultiFeatureNode::IGNORE);
/// assert!(MultiFeatureNode::FEATURES.contains(&FeatureId::TestV0_12_0Plus));
/// assert!(MultiFeatureNode::FEATURES.contains(&FeatureId::TestV0_12_5ToV0_13_0));
///
/// #[derive(DeriveFeature)]
/// #[feat(ignore)]
/// struct InternalNode;
///
/// assert!(InternalNode::IGNORE);
/// assert!(InternalNode::FEATURES.is_empty());
/// ```
#[proc_macro_derive(Feature, attributes(feat))]
pub fn derive_feature(input: TokenStream) -> TokenStream {
    feature::derive_feature(input)
}
