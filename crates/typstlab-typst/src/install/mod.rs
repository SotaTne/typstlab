pub mod platform;

// Re-export for convenience
pub use platform::{Arch, Os, asset_name_pattern, binary_name, detect_arch, detect_os};
