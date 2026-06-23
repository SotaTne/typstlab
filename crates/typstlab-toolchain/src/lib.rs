pub mod binary;
pub mod docs;
pub mod error;
pub mod platform;
pub mod runtime;
pub mod store;
pub mod toolchain_spec;
pub mod version_resolution;

pub use binary::{
    BinaryArchiveFormat, BinaryDistribution, BinaryLayout, BinaryTool, RawCommandFactory,
    ToolCommand, ToolInvocation, TypedBinaryTool, VersionCommand, VersionGuard,
    typst::TOOL_ID as TYPST_TOOL_ID,
};
pub use docs::{DocsSource, DocsSourceFormat, DocsTool, RenderedDocs};
pub use error::ToolchainError;
pub use platform::{Arch, Os, Platform};
pub use runtime::{ResolvedDocsTree, TypedResolvedBinary};
pub use store::{
    BinaryStoreKey, DocsStoreKey, StoredBinary, StoredDocsTree, ToolchainBinaryStore,
    ToolchainDocsStore, ToolchainStoreError,
};
pub use toolchain_spec::{ToolChoice, ToolchainCandidate, ToolchainSpec, ToolchainSpecError};
pub use version_resolution::{CompatibilityTable, VersionResolution, VersionResolveError};

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn every_toolchain_item_has_unique_id_across_kinds() {
        let mut ids = BTreeSet::new();

        for tool in crate::binary::TOOLS {
            assert!(
                ids.insert(tool.id()),
                "duplicate toolchain item id: {}",
                tool.id()
            );
        }

        for docs in crate::docs::DOCS {
            assert!(
                ids.insert(docs.id()),
                "duplicate toolchain item id: {}",
                docs.id()
            );
        }
    }
}
