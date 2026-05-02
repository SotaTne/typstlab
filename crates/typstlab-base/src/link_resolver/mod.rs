mod docs;
mod typst;

pub use docs::{DocsLinkRequest, resolve_docs_link};
pub use typst::{LinkResolveError, TypstLinkRequest, resolve_typst_link};

use typstlab_proto::SourceFormat;

/// Release version used for link resolution.
///
/// The value must be in `major.minor.patch` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version<'a>(&'a str);

impl<'a> Version<'a> {
    pub fn new(value: &'a str) -> Self {
        Self(value)
    }

    pub fn as_str(self) -> &'a str {
        self.0
    }
}

impl AsRef<str> for Version<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLink {
    pub url: String,
    pub format: SourceFormat,
}
