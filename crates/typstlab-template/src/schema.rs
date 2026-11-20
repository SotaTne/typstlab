use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use super::value::Value;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParamSchemas(BTreeMap<String, ParamSchema>);

impl ParamSchemas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_inner(self) -> BTreeMap<String, ParamSchema> {
        self.0
    }
}

impl From<BTreeMap<String, ParamSchema>> for ParamSchemas {
    fn from(value: BTreeMap<String, ParamSchema>) -> Self {
        Self(value)
    }
}

impl From<ParamSchemas> for BTreeMap<String, ParamSchema> {
    fn from(value: ParamSchemas) -> Self {
        value.into_inner()
    }
}

impl Deref for ParamSchemas {
    type Target = BTreeMap<String, ParamSchema>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ParamSchemas {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// template.tomlのinputにこれをおそらく使う
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSchema {
    #[serde(rename = "type")]
    pub value_type: ValueType,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub default: Option<ParamDefault>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValueType {
    String,
    Integer,
    Bool,
    Date,
    Array(Box<ValueType>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ParamDefault {
    Literal(Value),
    Source(ParamDefaultSource),
}

impl<'de> Deserialize<'de> for ParamDefault {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = toml::Value::deserialize(deserializer)?;
        if value.is_table() {
            return value
                .try_into()
                .map(Self::Source)
                .map_err(serde::de::Error::custom);
        }
        Value::from_toml(value)
            .map(Self::Literal)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamDefaultSource {
    Project(String),
    Context(String),
    Builtin(String),
}

fn default_required() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn param_schemas_deserializes_from_toml_table() {
        let schemas: ParamSchemas = toml::from_str(
            r#"
[title]
type = "string"

[date]
type = "date"
required = false
"#,
        )
        .unwrap();

        assert_eq!(schemas["title"].value_type, ValueType::String);
        assert_eq!(schemas["date"].value_type, ValueType::Date);
        assert!(!schemas["date"].required);
    }

    #[test]
    fn param_schemas_serializes_as_plain_map() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "title".to_string(),
            ParamSchema {
                value_type: ValueType::String,
                required: true,
                default: None,
            },
        );

        let toml = toml::to_string(&schemas).unwrap();

        assert!(toml.contains("[title]"));
        assert!(toml.contains("type = \"string\""));
    }

    #[test]
    fn round_trips_array_types_from_inline_tables() {
        for (input, item_type) in [
            (r#"type = { array = "string" }"#, ValueType::String),
            (
                r#"type = { array = { array = "date" } }"#,
                ValueType::Array(Box::new(ValueType::Date)),
            ),
        ] {
            let schema: ParamSchema = toml::from_str(input).unwrap();
            assert_eq!(schema.value_type, ValueType::Array(Box::new(item_type)));

            let encoded = toml::to_string(&schema).unwrap();
            assert_eq!(toml::from_str::<ParamSchema>(&encoded).unwrap(), schema);
            let original: toml::Table = toml::from_str(input).unwrap();
            let serialized: toml::Table = toml::from_str(&encoded).unwrap();
            assert_eq!(serialized["type"], original["type"]);
        }
    }

    #[test]
    fn round_trips_literal_defaults_with_native_dates_and_quoted_strings() {
        let date = Value::Date(NaiveDate::from_ymd_opt(2026, 5, 22).unwrap());
        for (value_type, default, expected) in [
            (r#""date""#, "2026-05-22", date.clone()),
            (
                r#""string""#,
                r#""2026-05-22""#,
                Value::String("2026-05-22".to_string()),
            ),
            (
                r#"{ array = { array = "date" } }"#,
                "[[2026-05-22]]",
                Value::Array(vec![Value::Array(vec![date])]),
            ),
        ] {
            let schema: ParamSchema =
                toml::from_str(&format!("type = {value_type}\ndefault = {default}")).unwrap();
            assert_eq!(schema.default, Some(ParamDefault::Literal(expected)));

            let encoded = toml::to_string(&schema).unwrap();
            assert!(encoded.contains(&format!("default = {default}")));
            assert_eq!(toml::from_str::<ParamSchema>(&encoded).unwrap(), schema);
        }
    }

    #[test]
    fn rejects_non_local_date_literal_defaults() {
        for value in ["12:00:00", "2026-05-22T12:00:00", "2026-05-22T12:00:00Z"] {
            let error =
                toml::from_str::<ParamSchema>(&format!("type = \"date\"\ndefault = {value}"))
                    .unwrap_err();
            assert!(error.message().contains("expected a TOML local date"));
        }
    }

    #[test]
    fn deserializes_default_source_from_dotted_key() {
        let schema: ParamSchema = toml::from_str(
            r#"
type = "string"
default.project = "author"
"#,
        )
        .unwrap();

        assert_eq!(
            schema.default,
            Some(ParamDefault::Source(ParamDefaultSource::Project(
                "author".to_string()
            )))
        );
        let encoded = toml::to_string(&schema).unwrap();
        assert_eq!(toml::from_str::<ParamSchema>(&encoded).unwrap(), schema);
    }

    #[test]
    fn deserializes_default_source_from_inline_table() {
        let schema: ParamSchema = toml::from_str(
            r#"
type = "date"
default = { builtin = "today" }
"#,
        )
        .unwrap();

        assert_eq!(
            schema.default,
            Some(ParamDefault::Source(ParamDefaultSource::Builtin(
                "today".to_string()
            )))
        );
        let encoded = toml::to_string(&schema).unwrap();
        assert_eq!(toml::from_str::<ParamSchema>(&encoded).unwrap(), schema);
    }
}
