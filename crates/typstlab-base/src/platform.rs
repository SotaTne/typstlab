use serde::{Deserialize, Serialize};

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
    Armv7,
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

        #[cfg(all(target_arch = "arm", target_feature = "v7"))]
        {
            Self::Armv7
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64",
            all(target_arch = "arm", target_feature = "v7")
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_current_is_supported() {
        let platform = Platform::current();

        assert!(matches!(platform.os, Os::Windows | Os::MacOS | Os::Linux));
    }

    #[test]
    fn test_platform_is_copyable_value() {
        let platform = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };

        let copied = platform;

        assert_eq!(copied, platform);
    }
}
