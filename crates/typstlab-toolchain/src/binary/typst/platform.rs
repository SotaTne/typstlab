use crate::{Arch, Os, Platform, SourceFormat, ToolchainError};

#[derive(Clone, Copy)]
pub struct TypstPlatformSpec {
    pub platform: Platform,
    pub asset_name: &'static str,
    pub source_format: SourceFormat,
    pub executable_relative_path: &'static str,
    pub archive_extension: &'static str,
}

impl TypstPlatformSpec {
    pub fn install_layout(&self) -> crate::InstallLayout {
        crate::InstallLayout {
            source_format: self.source_format,
            executable_relative_path: self.executable_relative_path.into(),
        }
    }
}

pub const SPECS: &[TypstPlatformSpec] = &[
    TypstPlatformSpec {
        platform: Platform {
            os: Os::MacOS,
            arch: Arch::X86_64,
        },
        asset_name: "typst-x86_64-apple-darwin",
        source_format: SourceFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    TypstPlatformSpec {
        platform: Platform {
            os: Os::MacOS,
            arch: Arch::Aarch64,
        },
        asset_name: "typst-aarch64-apple-darwin",
        source_format: SourceFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    TypstPlatformSpec {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        },
        asset_name: "typst-x86_64-unknown-linux-musl",
        source_format: SourceFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    TypstPlatformSpec {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::Aarch64,
        },
        asset_name: "typst-aarch64-unknown-linux-musl",
        source_format: SourceFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    TypstPlatformSpec {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::Riscv64,
        },
        asset_name: "typst-riscv64gc-unknown-linux-gnu",
        source_format: SourceFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    TypstPlatformSpec {
        platform: Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        },
        asset_name: "typst-x86_64-pc-windows-msvc",
        source_format: SourceFormat::Zip {
            strip_components: 1,
        },
        executable_relative_path: "typst.exe",
        archive_extension: "zip",
    },
    TypstPlatformSpec {
        platform: Platform {
            os: Os::Windows,
            arch: Arch::Aarch64,
        },
        asset_name: "typst-aarch64-pc-windows-msvc",
        source_format: SourceFormat::Zip {
            strip_components: 1,
        },
        executable_relative_path: "typst.exe",
        archive_extension: "zip",
    },
];

pub fn resolve(platform: Platform) -> Result<&'static TypstPlatformSpec, ToolchainError> {
    SPECS
        .iter()
        .find(|spec| spec.platform == platform)
        .ok_or(ToolchainError::UnsupportedPlatform {
            tool: "typst",
            platform,
        })
}
