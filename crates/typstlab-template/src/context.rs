use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::error::TemplateEngineError;
use super::source::{ParamSource, SourceValue};
use super::value::{Params, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitContext {
    #[serde(default)]
    pub template: Option<String>,
    pub date: NaiveDate,
}

impl InitContext {
    pub const TEMPLATE_KEY: &'static str = "template";
    pub const DATE_KEY: &'static str = "date";

    pub fn to_params(&self) -> Params {
        let mut params = Params::new();

        if let Some(template) = &self.template {
            params.insert(
                Self::TEMPLATE_KEY.to_string(),
                Value::String(template.clone()),
            );
        }

        params.insert(Self::DATE_KEY.to_string(), Value::Date(self.date));
        params
    }

    pub fn from_params(params: &Params) -> Result<Self, TemplateEngineError> {
        let template = match params.get(Self::TEMPLATE_KEY) {
            Some(Value::String(template)) => Some(template.clone()),
            Some(_) => {
                return Err(TemplateEngineError::InvalidParam {
                    name: Self::TEMPLATE_KEY.to_string(),
                    expected: "string".to_string(),
                });
            }
            None => None,
        };

        let date = match params.get(Self::DATE_KEY) {
            Some(Value::Date(date)) => *date,
            Some(_) => {
                return Err(TemplateEngineError::InvalidParam {
                    name: Self::DATE_KEY.to_string(),
                    expected: "date".to_string(),
                });
            }
            None => {
                return Err(TemplateEngineError::MissingParam(
                    Self::DATE_KEY.to_string(),
                ));
            }
        };

        Ok(Self { template, date })
    }
}

impl ParamSource for InitContext {
    fn resolve_param(&self, key: &str) -> SourceValue {
        match key {
            Self::TEMPLATE_KEY => self
                .template
                .as_ref()
                .cloned()
                .map(Value::String)
                .map(SourceValue::Found)
                .unwrap_or(SourceValue::Missing),
            Self::DATE_KEY => SourceValue::Found(Value::Date(self.date)),
            _ => SourceValue::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn converts_init_context_to_params() {
        let context = InitContext {
            template: Some("report".to_string()),
            date: date(2026, 5, 22),
        };

        let params = context.to_params();

        assert_eq!(
            params.get(InitContext::TEMPLATE_KEY),
            Some(&Value::String("report".to_string()))
        );
        assert_eq!(
            params.get(InitContext::DATE_KEY),
            Some(&Value::Date(date(2026, 5, 22)))
        );
    }

    #[test]
    fn omits_template_when_absent() {
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };

        let params = context.to_params();

        assert!(!params.contains_key(InitContext::TEMPLATE_KEY));
        assert_eq!(
            params.get(InitContext::DATE_KEY),
            Some(&Value::Date(date(2026, 5, 22)))
        );
    }

    #[test]
    fn converts_params_to_init_context() {
        let mut params = Params::new();
        params.insert(
            InitContext::TEMPLATE_KEY.to_string(),
            Value::String("report".to_string()),
        );
        params.insert(
            InitContext::DATE_KEY.to_string(),
            Value::Date(date(2026, 5, 22)),
        );

        assert_eq!(
            InitContext::from_params(&params).unwrap(),
            InitContext {
                template: Some("report".to_string()),
                date: date(2026, 5, 22),
            }
        );
    }

    #[test]
    fn requires_date_when_converting_from_params() {
        let error = InitContext::from_params(&Params::new()).unwrap_err();

        assert!(matches!(error, TemplateEngineError::MissingParam(name) if name == "date"));
    }

    #[test]
    fn rejects_invalid_template_type() {
        let mut params = Params::new();
        params.insert(InitContext::TEMPLATE_KEY.to_string(), Value::Bool(true));
        params.insert(
            InitContext::DATE_KEY.to_string(),
            Value::Date(date(2026, 5, 22)),
        );

        assert!(matches!(
            InitContext::from_params(&params).unwrap_err(),
            TemplateEngineError::InvalidParam { name, expected }
            if name == "template" && expected == "string"
        ));
    }

    #[test]
    fn rejects_invalid_date_type() {
        let mut params = Params::new();
        params.insert(
            InitContext::DATE_KEY.to_string(),
            Value::String("2026-05-22".to_string()),
        );

        assert!(matches!(
            InitContext::from_params(&params).unwrap_err(),
            TemplateEngineError::InvalidParam { name, expected }
            if name == "date" && expected == "date"
        ));
    }

    #[test]
    fn resolves_known_context_values() {
        let context = InitContext {
            template: Some("report".to_string()),
            date: date(2026, 5, 22),
        };

        assert_eq!(
            context.resolve_param(InitContext::TEMPLATE_KEY),
            SourceValue::Found(Value::String("report".to_string()))
        );
        assert_eq!(
            context.resolve_param(InitContext::DATE_KEY),
            SourceValue::Found(Value::Date(date(2026, 5, 22)))
        );
    }

    #[test]
    fn resolves_missing_optional_template() {
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };

        assert_eq!(
            context.resolve_param(InitContext::TEMPLATE_KEY),
            SourceValue::Missing
        );
    }

    #[test]
    fn resolves_unknown_context_key() {
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };

        assert_eq!(context.resolve_param("unknown"), SourceValue::Unknown);
    }
}
