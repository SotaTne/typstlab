pub mod from_static;
pub mod spec;

pub use from_static::{CompatibilityTable, VersionResolution, VersionResolveError};
pub use spec::{ToolChoice, ToolchainCandidate, ToolchainSpec, ToolchainSpecError};
