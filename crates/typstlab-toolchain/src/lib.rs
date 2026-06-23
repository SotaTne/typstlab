pub mod binary;
pub mod command;
pub mod docs;
pub mod error;
pub mod install;
pub mod platform;
pub mod resolver;
pub mod runtime;
pub mod store;

pub use binary::{BinaryTool, TypedBinaryTool, VersionCommand, typst::TOOL_ID as TYPST_TOOL_ID};
pub use command::{RawCommandFactory, ToolCommand, ToolInvocation, VersionGuard};
pub use docs::{DocsSource, DocsSourceFormat, DocsTool, RenderedDocs};
pub use error::ToolchainError;
pub use install::{InstallLayout, InstallSource, SourceFormat};
pub use platform::{Arch, Os, Platform};
pub use resolver::{
    CompatibilityTable, ToolChoice, ToolchainCandidate, ToolchainSpec, ToolchainSpecError,
    VersionResolution, VersionResolveError,
};
pub use runtime::{ResolvedDocsTree, TypedResolvedBinary};
pub use store::{
    BinaryStoreKey, DocsStoreKey, StoredBinary, StoredDocsTree, ToolchainBinaryStore,
    ToolchainDocsStore, ToolchainStoreError,
};

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
