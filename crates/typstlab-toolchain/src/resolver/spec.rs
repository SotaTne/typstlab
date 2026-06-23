use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Deref, DerefMut};

use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::binary::typst;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ToolchainSpecError {
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
pub struct ToolchainSpec(BTreeMap<String, ToolChoice>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainCandidate<'a> {
    pub id: &'a str,
    pub choice: &'a ToolChoice,
}

impl ToolchainSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn candidates(&self) -> Vec<ToolchainCandidate<'_>> {
        self.0
            .iter()
            .map(|(id, choice)| ToolchainCandidate {
                id: id.as_str(),
                choice,
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), ToolchainSpecError> {
        match self.0.get(typst::TOOL_ID) {
            Some(ToolChoice::Version(_)) => Ok(()),
            _ => Err(ToolchainSpecError::TypstVersionRequired),
        }
    }

    pub fn into_inner(self) -> BTreeMap<String, ToolChoice> {
        self.0
    }
}

impl From<BTreeMap<String, ToolChoice>> for ToolchainSpec {
    fn from(value: BTreeMap<String, ToolChoice>) -> Self {
        Self(value)
    }
}

impl From<ToolchainSpec> for BTreeMap<String, ToolChoice> {
    fn from(value: ToolchainSpec) -> Self {
        value.into_inner()
    }
}

impl Deref for ToolchainSpec {
    type Target = BTreeMap<String, ToolChoice>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ToolchainSpec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_choice_serializes_as_plain_string() {
        let serialized = serde_json::to_string(&ToolChoice::Version("0.14.2".to_string())).unwrap();

        assert_eq!(serialized, r#""0.14.2""#);
    }

    #[test]
    fn tool_choice_deserializes_special_values_and_versions() {
        assert_eq!(
            serde_json::from_str::<ToolChoice>(r#""auto""#).unwrap(),
            ToolChoice::Auto
        );
        assert_eq!(
            serde_json::from_str::<ToolChoice>(r#""none""#).unwrap(),
            ToolChoice::None
        );
        assert_eq!(
            serde_json::from_str::<ToolChoice>(r#""0.14.2""#).unwrap(),
            ToolChoice::Version("0.14.2".to_string())
        );
    }

    #[test]
    fn toolchain_spec_is_transparent_map() {
        let spec: ToolchainSpec = serde_json::from_str(
            r#"{
                "typst": "0.14.2",
                "typst-docs": "auto",
                "typstyle": "none"
            }"#,
        )
        .unwrap();

        assert_eq!(
            spec.get("typst"),
            Some(&ToolChoice::Version("0.14.2".to_string()))
        );
        assert_eq!(spec.get("typst-docs"), Some(&ToolChoice::Auto));
        assert_eq!(spec.get("typstyle"), Some(&ToolChoice::None));
    }

    #[test]
    fn toolchain_spec_returns_candidates_in_key_order() {
        let spec: ToolchainSpec = serde_json::from_str(
            r#"{
                "typstyle": "none",
                "typst": "0.14.2",
                "typst-docs": "auto"
            }"#,
        )
        .unwrap();

        let candidates = spec.candidates();

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
    fn toolchain_spec_requires_explicit_typst_version() {
        let spec: ToolchainSpec = serde_json::from_str(
            r#"{
                "typst": "auto"
            }"#,
        )
        .unwrap();

        assert_eq!(
            spec.validate(),
            Err(ToolchainSpecError::TypstVersionRequired)
        );
    }

    #[test]
    fn toolchain_spec_accepts_explicit_typst_version() {
        let spec: ToolchainSpec = serde_json::from_str(
            r#"{
                "typst": "0.14.2"
            }"#,
        )
        .unwrap();

        assert_eq!(spec.validate(), Ok(()));
    }
}
