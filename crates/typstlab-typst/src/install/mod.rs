pub mod platform;
pub mod release;
pub mod select;

// Re-export for convenience
pub use platform::{Arch, Os, asset_name_pattern, binary_name, detect_arch, detect_os};
pub use release::{Asset, Release, ReleaseError, fetch_release_metadata};
pub use select::{select_asset, select_asset_for_current_platform};
