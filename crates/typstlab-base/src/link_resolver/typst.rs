use super::{ResolvedLink, Version};
use crate::platform::{Arch, Os, Platform};
use thiserror::Error;
use typstlab_proto::SourceFormat;

const TYPST_TAR_XZ_FORMAT: SourceFormat = SourceFormat::TarXz {
    strip_components: 1,
};
const TYPST_ZIP_FORMAT: SourceFormat = SourceFormat::Zip {
    strip_components: 1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstLinkRequest<'a> {
    pub platform: Platform,
    pub version: Version<'a>,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum LinkResolveError {
    #[error("unsupported platform for typst download: {platform:?}")]
    UnsupportedTypstPlatform { platform: Platform },
}

pub fn resolve_typst_link(request: TypstLinkRequest<'_>) -> Result<ResolvedLink, LinkResolveError> {
    let (target, format) = typst_target_and_format(request.platform)?;
    Ok(ResolvedLink {
        url: format!(
            "https://github.com/typst/typst/releases/download/v{}/typst-{}.{}",
            request.version.as_str(),
            target,
            typst_archive_extension(request.platform.os)
        ),
        format,
    })
}

fn typst_target_and_format(
    platform: Platform,
) -> Result<(&'static str, SourceFormat), LinkResolveError> {
    let format = match platform.os {
        Os::Windows => TYPST_ZIP_FORMAT,
        Os::MacOS | Os::Linux => TYPST_TAR_XZ_FORMAT,
    };

    let target = match (platform.os, platform.arch) {
        (Os::MacOS, Arch::X86_64) => "x86_64-apple-darwin",
        (Os::MacOS, Arch::Aarch64) => "aarch64-apple-darwin",
        (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux-musl",
        (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux-musl",
        (Os::Windows, Arch::X86_64) => "x86_64-pc-windows-msvc",
        (Os::Windows, Arch::Aarch64) => "aarch64-pc-windows-msvc",
        _ => return Err(LinkResolveError::UnsupportedTypstPlatform { platform }),
    };

    Ok((target, format))
}

fn typst_archive_extension(os: Os) -> &'static str {
    match os {
        Os::Windows => "zip",
        Os::MacOS | Os::Linux => "tar.xz",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(os: Os, arch: Arch) -> TypstLinkRequest<'static> {
        TypstLinkRequest {
            platform: Platform { os, arch },
            version: Version::new("0.14.2"),
        }
    }

    #[test]
    fn test_resolve_typst_link_for_macos_x86_64() {
        let link = resolve_typst_link(request(Os::MacOS, Arch::X86_64)).unwrap();

        assert_eq!(
            link.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-x86_64-apple-darwin.tar.xz"
        );
        assert_eq!(
            link.format,
            SourceFormat::TarXz {
                strip_components: 1
            }
        );
    }

    #[test]
    fn test_resolve_typst_link_for_linux_x86_64() {
        let link = resolve_typst_link(request(Os::Linux, Arch::X86_64)).unwrap();

        assert_eq!(
            link.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-x86_64-unknown-linux-musl.tar.xz"
        );
        assert_eq!(
            link.format,
            SourceFormat::TarXz {
                strip_components: 1
            }
        );
    }

    #[test]
    fn test_resolve_typst_link_for_linux_aarch64() {
        let link = resolve_typst_link(request(Os::Linux, Arch::Aarch64)).unwrap();

        assert_eq!(
            link.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-aarch64-unknown-linux-musl.tar.xz"
        );
        assert_eq!(
            link.format,
            SourceFormat::TarXz {
                strip_components: 1
            }
        );
    }

    #[test]
    fn test_resolve_typst_link_for_windows_x86_64() {
        let link = resolve_typst_link(request(Os::Windows, Arch::X86_64)).unwrap();

        assert_eq!(
            link.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            link.format,
            SourceFormat::Zip {
                strip_components: 1
            }
        );
    }

    #[test]
    fn test_resolve_typst_link_for_windows_aarch64() {
        let link = resolve_typst_link(request(Os::Windows, Arch::Aarch64)).unwrap();

        assert_eq!(
            link.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-aarch64-pc-windows-msvc.zip"
        );
        assert_eq!(
            link.format,
            SourceFormat::Zip {
                strip_components: 1
            }
        );
    }

    #[test]
    fn test_resolve_typst_link_rejects_unsupported_arch() {
        let err = resolve_typst_link(request(Os::Linux, Arch::Riscv64)).unwrap_err();

        assert!(matches!(
            err,
            LinkResolveError::UnsupportedTypstPlatform { .. }
        ));
    }
}
