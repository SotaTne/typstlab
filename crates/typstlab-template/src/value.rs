use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use toml::value::Datetime;

use crate::TemplateEngineError;

/// typstlab.tomlのparams,paper.tomlのinit.paramにこれを使う
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(BTreeMap<String, Value>);

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_inner(self) -> BTreeMap<String, Value> {
        self.0
    }
}

impl From<BTreeMap<String, Value>> for Params {
    fn from(value: BTreeMap<String, Value>) -> Self {
        Self(value)
    }
}

impl From<Params> for BTreeMap<String, Value> {
    fn from(value: Params) -> Self {
        value.into_inner()
    }
}

impl Deref for Params {
    type Target = BTreeMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Params {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Integer(i64),
    Bool(bool),
    Date(NaiveDate),
    Array(Vec<Value>),
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::String(value) => serializer.serialize_str(value),
            Self::Integer(value) => serializer.serialize_i64(*value),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Date(value) => value
                .to_string()
                .parse::<Datetime>()
                .map_err(serde::ser::Error::custom)?
                .serialize(serializer),
            Self::Array(values) => values.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_toml(toml::Value::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Value {
    pub(crate) fn from_toml(value: toml::Value) -> Result<Self, &'static str> {
        match value {
            toml::Value::String(value) => Ok(Self::String(value)),
            toml::Value::Integer(value) => Ok(Self::Integer(value)),
            toml::Value::Boolean(value) => Ok(Self::Bool(value)),
            toml::Value::Datetime(Datetime {
                date: Some(date),
                time: None,
                offset: None,
            }) => NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())
                .map(Self::Date)
                .ok_or("invalid TOML local date"),
            toml::Value::Datetime(_) => Err("expected a TOML local date without a time or offset"),
            toml::Value::Array(values) => values
                .into_iter()
                .map(Self::from_toml)
                .collect::<Result<_, _>>()
                .map(Self::Array),
            _ => Err("expected a string, integer, boolean, local date, or array"),
        }
    }

    pub fn to_display_text(&self) -> Result<String, TemplateEngineError> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Date(d) => Ok(d.to_string()),
            Value::Array(_) => Err(TemplateEngineError::NonDisplayableValue),
        }
    }

    /// Renders a scalar as a Typst display expression for use in markup.
    pub fn to_typst_display(&self) -> Result<String, TemplateEngineError> {
        Ok(format!("#({})", typst_string(&self.to_display_text()?)))
    }

    pub fn to_typst_literal(&self) -> Result<String, TemplateEngineError> {
        match self {
            Value::String(s) => Ok(typst_string(s)),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Date(d) => Ok(format!(
                "datetime(year: {}, month: {}, day: {})",
                d.year(),
                d.month(),
                d.day()
            )),
            Value::Array(values) => {
                let len = values.len();
                let values = values
                    .iter()
                    .map(Value::to_typst_literal)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                match len {
                    0 => Ok("()".to_string()),
                    1 => Ok(format!("({values},)")),
                    _ => Ok(format!("({values})")),
                }
            }
        }
    }
}

fn typst_string(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');
    for ch in value.chars() {
        match ch {
            '"' => literal.push_str(r#"\""#),
            '\\' => literal.push_str(r"\\"),
            '\n' => literal.push_str(r"\n"),
            '\r' => literal.push_str(r"\r"),
            '\t' => literal.push_str(r"\t"),
            ch if ch.is_control() => literal.extend(ch.escape_unicode()),
            ch => literal.push(ch),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn params_round_trip_native_dates_and_quoted_strings() {
        let params: Params = toml::from_str(
            r#"
date = 2026-05-22
text = "2026-05-22"
items = [2026-05-22, "2026-05-22", [2000-02-29]]
"#,
        )
        .unwrap();
        assert_eq!(params["date"], Value::Date(date(2026, 5, 22)));
        assert_eq!(params["text"], Value::String("2026-05-22".to_string()));
        assert_eq!(
            params["items"],
            Value::Array(vec![
                params["date"].clone(),
                params["text"].clone(),
                Value::Array(vec![Value::Date(date(2000, 2, 29))]),
            ])
        );

        let encoded = toml::to_string(&params).unwrap();
        assert!(encoded.contains("date = 2026-05-22"));
        assert!(encoded.contains(r#"text = "2026-05-22""#));
        assert_eq!(toml::from_str::<Params>(&encoded).unwrap(), params);
    }

    #[test]
    fn rejects_toml_times_and_datetimes_inside_or_outside_arrays() {
        for value in [
            "12:00:00",
            "2026-05-22T12:00:00",
            "2026-05-22T12:00:00Z",
            "2026-05-22T12:00:00+09:00",
        ] {
            for input in [format!("value = {value}"), format!("value = [{value}]")] {
                let error = toml::from_str::<Params>(&input).unwrap_err();
                assert!(error.message().contains("expected a TOML local date"));
            }
        }
    }

    #[test]
    fn rejects_serializing_dates_outside_the_toml_year_range() {
        let mut params = Params::new();
        for year in [-1, 10_000] {
            params.insert("date".to_string(), Value::Date(date(year, 1, 1)));
            assert!(toml::to_string(&params).is_err());
        }
    }

    #[test]
    fn string_display_text_is_raw_text() {
        let value = Value::String("Report 01".to_string());

        assert_eq!(value.to_display_text().unwrap(), "Report 01");
    }

    #[test]
    fn scalar_display_text_uses_plain_strings() {
        assert_eq!(Value::Integer(42).to_display_text().unwrap(), "42");
        assert_eq!(Value::Integer(-7).to_display_text().unwrap(), "-7");
        assert_eq!(Value::Bool(true).to_display_text().unwrap(), "true");
        assert_eq!(Value::Bool(false).to_display_text().unwrap(), "false");
        assert_eq!(
            Value::Date(date(2026, 5, 21)).to_display_text().unwrap(),
            "2026-05-21"
        );
    }

    #[test]
    fn array_cannot_be_display_text() {
        let error = Value::Array(vec![Value::String("Hoge".to_string())])
            .to_display_text()
            .unwrap_err();

        assert!(matches!(error, TemplateEngineError::NonDisplayableValue));
    }

    #[test]
    fn string_typst_literal_is_quoted_and_escaped() {
        let value =
            Value::String("*_#[]<>@$\\// /* */ \" ) #panic()\n\r\t\0\u{8}\u{c}日本語".to_string());
        let literal = r#""*_#[]<>@$\\// /* */ \" ) #panic()\n\r\t\u{0}\u{8}\u{c}日本語""#;

        assert_eq!(value.to_typst_literal().unwrap(), literal);
        assert_eq!(value.to_typst_display().unwrap(), format!("#({literal})"));
    }

    #[test]
    fn scalar_typst_literals_use_typst_syntax() {
        assert_eq!(Value::Integer(42).to_typst_literal().unwrap(), "42");
        assert_eq!(Value::Integer(-7).to_typst_literal().unwrap(), "-7");
        assert_eq!(Value::Bool(true).to_typst_literal().unwrap(), "true");
        assert_eq!(Value::Bool(false).to_typst_literal().unwrap(), "false");
        assert_eq!(
            Value::Date(date(2026, 5, 21)).to_typst_literal().unwrap(),
            "datetime(year: 2026, month: 5, day: 21)"
        );
    }

    #[test]
    fn empty_array_typst_literal_is_empty_typst_array() {
        assert_eq!(Value::Array(vec![]).to_typst_literal().unwrap(), "()");
    }

    #[test]
    fn single_item_array_typst_literal_has_trailing_comma() {
        let value = Value::Array(vec![Value::String("Hoge".to_string())]);

        assert_eq!(value.to_typst_literal().unwrap(), r#"("Hoge",)"#);
    }

    #[test]
    fn multiple_item_array_typst_literal_is_comma_separated() {
        let value = Value::Array(vec![
            Value::String("Hoge".to_string()),
            Value::String("ChatGPT".to_string()),
            Value::Integer(3),
        ]);

        assert_eq!(
            value.to_typst_literal().unwrap(),
            r#"("Hoge", "ChatGPT", 3)"#
        );
    }

    #[test]
    fn nested_array_typst_literal_is_recursive() {
        let value = Value::Array(vec![
            Value::Array(vec![Value::String("Hoge".to_string())]),
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
            Value::Array(vec![]),
        ]);

        assert_eq!(
            value.to_typst_literal().unwrap(),
            r#"(("Hoge",), (1, 2), ())"#
        );
    }

    #[test]
    fn array_string_with_comma_does_not_affect_array_length() {
        let value = Value::Array(vec![Value::String("Hoge, ChatGPT".to_string())]);

        assert_eq!(value.to_typst_literal().unwrap(), r#"("Hoge, ChatGPT",)"#);
    }
}
