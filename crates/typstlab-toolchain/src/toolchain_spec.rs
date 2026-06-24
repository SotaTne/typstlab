use serde::{Deserialize, Serialize};

use crate::{ToolChoices, ToolChoicesError};

/// typstlab の toolchain 設定全体。
///
/// binary tools と docs は registry が別なので、TOML でも [tools] / [docs] として分ける。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ToolchainSpec {
    #[serde(default)]
    pub tools: ToolChoices,
    #[serde(default)]
    pub docs: ToolChoices,
}

impl ToolchainSpec {
    pub fn validate(&self) -> Result<(), ToolChoicesError> {
        self.tools.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolChoice;

    #[test]
    fn toolchain_spec_deserializes_tools_and_docs_sections() {
        let spec: ToolchainSpec = toml::from_str(
            r#"
                [tools]
                typst = "0.14.2"
                typstyle = "none"

                [docs]
                typst-docs = "auto"
            "#,
        )
        .unwrap();

        assert_eq!(
            spec.tools.get("typst"),
            Some(&ToolChoice::Version("0.14.2".to_string()))
        );
        assert_eq!(spec.tools.get("typstyle"), Some(&ToolChoice::None));
        assert_eq!(spec.docs.get("typst-docs"), Some(&ToolChoice::Auto));
    }

    #[test]
    fn toolchain_spec_requires_explicit_typst_tool_version() {
        let spec: ToolchainSpec = toml::from_str(
            r#"
                [tools]
                typst = "auto"
            "#,
        )
        .unwrap();

        assert_eq!(spec.validate(), Err(ToolChoicesError::TypstVersionRequired));
    }
}
