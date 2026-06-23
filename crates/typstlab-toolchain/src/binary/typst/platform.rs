use crate::binary::PlatformAssetSpec;
use crate::{Arch, Os, Platform, BinaryArchiveFormat};

pub const SPECS: &[PlatformAssetSpec] = &[
    PlatformAssetSpec {
        platform: Platform {
            os: Os::MacOS,
            arch: Arch::X86_64,
        },
        asset_name: "typst-x86_64-apple-darwin",
        archive_format: BinaryArchiveFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    PlatformAssetSpec {
        platform: Platform {
            os: Os::MacOS,
            arch: Arch::Aarch64,
        },
        asset_name: "typst-aarch64-apple-darwin",
        archive_format: BinaryArchiveFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    PlatformAssetSpec {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        },
        asset_name: "typst-x86_64-unknown-linux-musl",
        archive_format: BinaryArchiveFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    PlatformAssetSpec {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::Aarch64,
        },
        asset_name: "typst-aarch64-unknown-linux-musl",
        archive_format: BinaryArchiveFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    PlatformAssetSpec {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::Riscv64,
        },
        asset_name: "typst-riscv64gc-unknown-linux-gnu",
        archive_format: BinaryArchiveFormat::TarXz {
            strip_components: 1,
        },
        executable_relative_path: "typst",
        archive_extension: "tar.xz",
    },
    PlatformAssetSpec {
        platform: Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        },
        asset_name: "typst-x86_64-pc-windows-msvc",
        archive_format: BinaryArchiveFormat::Zip {
            strip_components: 1,
        },
        executable_relative_path: "typst.exe",
        archive_extension: "zip",
    },
    PlatformAssetSpec {
        platform: Platform {
            os: Os::Windows,
            arch: Arch::Aarch64,
        },
        asset_name: "typst-aarch64-pc-windows-msvc",
        archive_format: BinaryArchiveFormat::Zip {
            strip_components: 1,
        },
        executable_relative_path: "typst.exe",
        archive_extension: "zip",
    },
];
