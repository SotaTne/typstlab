mod close_cfg;
mod close_deriver;
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
    close_deriver::derive_close(input)
}
