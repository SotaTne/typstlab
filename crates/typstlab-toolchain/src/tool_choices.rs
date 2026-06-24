/*
 * これは、tool id ごとの選択肢を TOML として読み書きするためのモジュールです。
 * tools/docs の分類や実際の version 解決は扱わず、ToolChoice の集合だけを保持します。
 */

use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Deref, DerefMut};

use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::ToolchainTool;
use crate::binary::typst;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ToolChoicesError {
    #[error("typst must be configured with an explicit version")]
    TypstVersionRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolChoice {
    Auto,
    None,
    Version(String),
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Auto => serializer.serialize_str("auto"),
            Self::None => serializer.serialize_str("none"),
            Self::Version(version) => serializer.serialize_str(version),
        }
    }
}

impl<'de> Deserialize<'de> for ToolChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ToolChoiceVisitor)
    }
}

struct ToolChoiceVisitor;

impl Visitor<'_> for ToolChoiceVisitor {
    type Value = ToolChoice;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(r#""auto", "none", or a version string"#)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(match value {
            "auto" => ToolChoice::Auto,
            "none" => ToolChoice::None,
            version => ToolChoice::Version(version.to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolChoices(BTreeMap<String, ToolChoice>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolChoiceCandidate<'a> {
    pub id: &'a str,
    pub choice: &'a ToolChoice,
}

impl ToolChoices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn candidates(&self) -> Vec<ToolChoiceCandidate<'_>> {
        self.0
            .iter()
            .map(|(id, choice)| ToolChoiceCandidate {
                id: id.as_str(),
                choice,
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), ToolChoicesError> {
        match self.0.get(typst::TOOL.id()) {
            Some(ToolChoice::Version(_)) => Ok(()),
            _ => Err(ToolChoicesError::TypstVersionRequired),
        }
    }

    pub fn into_inner(self) -> BTreeMap<String, ToolChoice> {
        self.0
    }
}

impl From<BTreeMap<String, ToolChoice>> for ToolChoices {
    fn from(value: BTreeMap<String, ToolChoice>) -> Self {
        Self(value)
    }
}

impl From<ToolChoices> for BTreeMap<String, ToolChoice> {
    fn from(value: ToolChoices) -> Self {
        value.into_inner()
    }
}

impl Deref for ToolChoices {
    type Target = BTreeMap<String, ToolChoice>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ToolChoices {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct ChoiceWrapper {
        choice: ToolChoice,
    }

    #[test]
    fn tool_choice_serializes_as_plain_string() {
        let serialized = toml::to_string(&ChoiceWrapper {
            choice: ToolChoice::Version("0.14.2".to_string()),
        })
        .unwrap();

        assert_eq!(serialized.trim(), r#"choice = "0.14.2""#);
    }

    #[test]
    fn tool_choice_deserializes_special_values_and_versions() {
        assert_eq!(
            toml::from_str::<ChoiceWrapper>("choice = \"auto\"")
                .unwrap()
                .choice,
            ToolChoice::Auto
        );
        assert_eq!(
            toml::from_str::<ChoiceWrapper>("choice = \"none\"")
                .unwrap()
                .choice,
            ToolChoice::None
        );
        assert_eq!(
            toml::from_str::<ChoiceWrapper>("choice = \"0.14.2\"")
                .unwrap()
                .choice,
            ToolChoice::Version("0.14.2".to_string())
        );
    }

    #[test]
    fn tool_choices_is_transparent_map() {
        let choices: ToolChoices = toml::from_str(
            r#"
                typst = "0.14.2"
                typst-docs = "auto"
                typstyle = "none"
            "#,
        )
        .unwrap();

        assert_eq!(
            choices.get("typst"),
            Some(&ToolChoice::Version("0.14.2".to_string()))
        );
        assert_eq!(choices.get("typst-docs"), Some(&ToolChoice::Auto));
        assert_eq!(choices.get("typstyle"), Some(&ToolChoice::None));
    }

    #[test]
    fn tool_choices_returns_candidates_in_key_order() {
        let choices: ToolChoices = toml::from_str(
            r#"
                typstyle = "none"
                typst = "0.14.2"
                typst-docs = "auto"
            "#,
        )
        .unwrap();

        let candidates = choices.candidates();

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>(),
            ["typst", "typst-docs", "typstyle"]
        );
        assert_eq!(candidates[1].choice, &ToolChoice::Auto);
    }

    #[test]
    fn tool_choices_requires_explicit_typst_version() {
        let choices: ToolChoices = toml::from_str(
            r#"
                typst = "auto"
            "#,
        )
        .unwrap();

        assert_eq!(
            choices.validate(),
            Err(ToolChoicesError::TypstVersionRequired)
        );
    }

    #[test]
    fn tool_choices_accepts_explicit_typst_version() {
        let choices: ToolChoices = toml::from_str(
            r#"
                typst = "0.14.2"
            "#,
        )
        .unwrap();

        assert_eq!(choices.validate(), Ok(()));
    }
}
