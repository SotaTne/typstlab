pub mod platform;
pub mod release;
mod select; // TDD: tests only, will be made public after implementation

// Re-export for convenience
pub use platform::{Arch, Os, asset_name_pattern, binary_name, detect_arch, detect_os};
pub use release::{Asset, Release, ReleaseError, fetch_release_metadata};
