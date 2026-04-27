mod docs;
mod typst;

pub use docs::{DocsLinkRequest, resolve_docs_link};
pub use typst::{LinkResolveError, TypstLinkRequest, resolve_typst_link};

use typstlab_proto::SourceFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLink {
    pub url: String,
    pub format: SourceFormat,
}
