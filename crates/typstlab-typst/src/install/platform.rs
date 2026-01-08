use crate::Result;
#[cfg(not(all(
    any(target_os = "macos", target_os = "linux", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
use typstlab_core::error::TypstlabError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOS,
    Linux,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

pub fn detect_os() -> Result<Os> {
    #[cfg(target_os = "macos")]
    return Ok(Os::MacOS);

    #[cfg(target_os = "linux")]
    return Ok(Os::Linux);

    #[cfg(target_os = "windows")]
    return Ok(Os::Windows);

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    Err(TypstlabError::TypstInstallFailed(format!(
        "Unsupported operating system: {}",
        std::env::consts::OS
    )))
}

pub fn detect_arch() -> Result<Arch> {
    #[cfg(target_arch = "x86_64")]
    return Ok(Arch::X86_64);

    #[cfg(target_arch = "aarch64")]
    return Ok(Arch::Aarch64);

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    Err(TypstlabError::TypstInstallFailed(format!(
        "Unsupported architecture: {}",
        std::env::consts::ARCH
    )))
}

pub fn asset_name_pattern(os: Os, arch: Arch) -> String {
    match (os, arch) {
        (Os::MacOS, Arch::X86_64) => "x86_64-apple-darwin",
        (Os::MacOS, Arch::Aarch64) => "aarch64-apple-darwin",
        (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux",
        (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux",
        (Os::Windows, Arch::X86_64) => "x86_64-pc-windows",
        (Os::Windows, Arch::Aarch64) => "aarch64-pc-windows",
    }
    .to_string()
}

pub fn binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "typst.exe";

    #[cfg(not(target_os = "windows"))]
    return "typst";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_os_succeeds() {
        let os = detect_os();
        assert!(os.is_ok(), "detect_os should succeed on current platform");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_os_macos() {
        assert_eq!(detect_os().unwrap(), Os::MacOS);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_detect_os_linux() {
        assert_eq!(detect_os().unwrap(), Os::Linux);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_detect_os_windows() {
        assert_eq!(detect_os().unwrap(), Os::Windows);
    }

    #[test]
    fn test_detect_arch_succeeds() {
        let arch = detect_arch();
        assert!(
            arch.is_ok(),
            "detect_arch should succeed on current platform"
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_detect_arch_x86_64() {
        assert_eq!(detect_arch().unwrap(), Arch::X86_64);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_detect_arch_aarch64() {
        assert_eq!(detect_arch().unwrap(), Arch::Aarch64);
    }

    #[test]
    fn test_asset_name_pattern_macos_x86() {
        let pattern = asset_name_pattern(Os::MacOS, Arch::X86_64);
        assert_eq!(pattern, "x86_64-apple-darwin");
    }

    #[test]
    fn test_asset_name_pattern_macos_aarch64() {
        let pattern = asset_name_pattern(Os::MacOS, Arch::Aarch64);
        assert_eq!(pattern, "aarch64-apple-darwin");
    }

    #[test]
    fn test_asset_name_pattern_linux_x86() {
        let pattern = asset_name_pattern(Os::Linux, Arch::X86_64);
        assert_eq!(pattern, "x86_64-unknown-linux");
    }

    #[test]
    fn test_asset_name_pattern_linux_aarch64() {
        let pattern = asset_name_pattern(Os::Linux, Arch::Aarch64);
        assert_eq!(pattern, "aarch64-unknown-linux");
    }

    #[test]
    fn test_asset_name_pattern_windows_x86() {
        let pattern = asset_name_pattern(Os::Windows, Arch::X86_64);
        assert_eq!(pattern, "x86_64-pc-windows");
    }

    #[test]
    fn test_asset_name_pattern_windows_aarch64() {
        let pattern = asset_name_pattern(Os::Windows, Arch::Aarch64);
        assert_eq!(pattern, "aarch64-pc-windows");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_binary_name_windows() {
        assert_eq!(binary_name(), "typst.exe");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_binary_name_unix() {
        assert_eq!(binary_name(), "typst");
    }
}
