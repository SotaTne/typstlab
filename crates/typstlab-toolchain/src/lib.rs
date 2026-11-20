pub mod binary;
pub mod docs;
pub mod error;
pub mod platform;
pub mod registry;
pub mod resolver;
pub mod tool;
pub mod tool_choices;
pub mod toolchain_spec;
pub mod version_resolution;

pub use binary::{
    BinaryArchiveFormat, BinaryDistribution, BinaryLayout, BinaryTool, RawCommandFactory,
    ToolCommand, ToolInvocation, TypedBinaryTool, VersionCommand, VersionGuard,
};
pub use docs::{DocsSource, DocsSourceFormat, DocsTool};
pub use error::ToolchainError;
pub use platform::{Arch, Os, Platform};
pub use registry::{BINARY_TOOLS, DOCS_TOOLS};
pub use resolver::{ResolvedToolchainSpec, ToolchainResolveError, resolve_toolchain};
pub use tool::ToolchainTool;
pub use tool_choices::{ToolChoice, ToolChoiceCandidate, ToolChoices};
pub use toolchain_spec::{ToolchainSpec, ToolchainSpecError};
pub use version_resolution::{CompatibilityTable, VersionResolution, VersionResolveError};
