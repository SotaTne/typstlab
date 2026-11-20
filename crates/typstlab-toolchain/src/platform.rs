use serde::{Deserialize, Serialize};

use crate::ToolchainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Os {
    Windows,
    MacOS,
    Linux,
}

impl Os {
    pub const fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }

        #[cfg(target_os = "macos")]
        {
            Self::MacOS
        }

        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            compile_error!("Unsupported OS");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arch {
    X86_64,
    Aarch64,
    Riscv64,
}

impl Arch {
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }

        #[cfg(target_arch = "aarch64")]
        {
            Self::Aarch64
        }

        #[cfg(target_arch = "riscv64")]
        {
            Self::Riscv64
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            compile_error!("Unsupported architecture");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub const fn current() -> Self {
        Self {
            os: Os::current(),
            arch: Arch::current(),
        }
    }

    pub fn target_triple(self) -> Result<&'static str, ToolchainError> {
        match (self.os, self.arch) {
            (Os::MacOS, Arch::X86_64) => Ok("x86_64-apple-darwin"),
            (Os::MacOS, Arch::Aarch64) => Ok("aarch64-apple-darwin"),
            (Os::Linux, Arch::X86_64) => Ok("x86_64-unknown-linux-musl"),
            (Os::Linux, Arch::Aarch64) => Ok("aarch64-unknown-linux-musl"),
            (Os::Linux, Arch::Riscv64) => Ok("riscv64gc-unknown-linux-gnu"),
            (Os::Windows, Arch::X86_64) => Ok("x86_64-pc-windows-msvc"),
            (Os::Windows, Arch::Aarch64) => Ok("aarch64-pc-windows-msvc"),
            (Os::MacOS | Os::Windows, Arch::Riscv64) => Err(ToolchainError::UnsupportedPlatform {
                tool: "target triple",
                platform: self,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_triples_match_supported_release_targets() {
        for (os, arch, expected) in [
            (Os::MacOS, Arch::X86_64, "x86_64-apple-darwin"),
            (Os::MacOS, Arch::Aarch64, "aarch64-apple-darwin"),
            (Os::Linux, Arch::X86_64, "x86_64-unknown-linux-musl"),
            (Os::Linux, Arch::Aarch64, "aarch64-unknown-linux-musl"),
            (Os::Linux, Arch::Riscv64, "riscv64gc-unknown-linux-gnu"),
            (Os::Windows, Arch::X86_64, "x86_64-pc-windows-msvc"),
            (Os::Windows, Arch::Aarch64, "aarch64-pc-windows-msvc"),
        ] {
            assert_eq!(Platform { os, arch }.target_triple().unwrap(), expected);
        }

        for os in [Os::MacOS, Os::Windows] {
            let platform = Platform {
                os,
                arch: Arch::Riscv64,
            };
            assert!(matches!(
                platform.target_triple(),
                Err(ToolchainError::UnsupportedPlatform { platform: rejected, .. })
                    if rejected == platform
            ));
        }
    }
}
