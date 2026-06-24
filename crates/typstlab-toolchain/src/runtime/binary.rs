use std::path::PathBuf;

use crate::{BinaryLayout, Platform, StoredBinary, ToolchainError, TypedBinaryTool};

#[derive(Debug, Clone)]
pub struct ResolvedBinary<'a, T>
where
    T: TypedBinaryTool,
{
    pub tool: &'a T,
    pub stored: StoredBinary,
    pub version: String,
    pub layout: BinaryLayout,
}

impl<'a, T> ResolvedBinary<'a, T>
where
    T: TypedBinaryTool,
{
    pub fn new(
        tool: &'a T,
        stored: StoredBinary,
        version: impl Into<String>,
        platform: Platform,
    ) -> Result<Self, ToolchainError> {
        let layout = tool.binary_layout(platform)?;
        Ok(Self {
            tool,
            stored,
            version: version.into(),
            layout,
        })
    }

    pub fn executable_path(&self) -> PathBuf {
        self.stored
            .install_root
            .join(&self.layout.executable_relative_path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::binary::typst;
    use crate::{Arch, Os, Platform, ResolvedBinary, StoredBinary, ToolchainTool};

    #[test]
    fn resolved_binary_resolves_executable_path_from_layout() {
        let stored = StoredBinary {
            install_root: "store/typst".into(),
        };
        let resolved = ResolvedBinary::new(
            &typst::TOOL,
            stored,
            "0.14.2",
            Platform {
                os: Os::MacOS,
                arch: Arch::Aarch64,
            },
        )
        .unwrap();

        assert_eq!(resolved.tool.id(), "typst");
        assert_eq!(resolved.version, "0.14.2");
        assert_eq!(
            resolved.executable_path(),
            PathBuf::from("store/typst").join("typst")
        );
    }

    #[test]
    fn resolved_binary_resolves_windows_executable_path_from_layout() {
        let stored = StoredBinary {
            install_root: "store/typst".into(),
        };
        let resolved = ResolvedBinary::new(
            &typst::TOOL,
            stored,
            "0.14.2",
            Platform {
                os: Os::Windows,
                arch: Arch::X86_64,
            },
        )
        .unwrap();

        assert_eq!(
            resolved.executable_path(),
            PathBuf::from("store/typst").join("typst.exe")
        );
    }
}
