use super::builtin::BuiltinContext;
use super::context::InitContext;
use super::error::TemplateEngineError;
use super::schema::{ParamDefault, ParamDefaultSource, ParamSchema, ParamSchemas, ValueType};
use super::source::{ParamSource, SourceValue};
use super::value::{Params, Value};

pub struct ParamResolver<'a> {
    pub schemas: &'a ParamSchemas,
    pub sources: ParamSources<'a>,
}

pub struct ParamSources<'a> {
    pub provided: &'a Params,
    pub project: &'a Params,
    pub context: &'a InitContext,
    pub builtin: &'a BuiltinContext,
}

impl<'a> ParamResolver<'a> {
    pub fn resolve(&self) -> Result<Params, TemplateEngineError> {
        self.validate_provided_keys()?;

        let mut resolved = Params::new();

        for (name, schema) in self.schemas.iter() {
            if let Some(value) = self.resolve_one(name, schema)? {
                resolved.insert(name.clone(), value);
            }
        }

        Ok(resolved)
    }

    fn validate_provided_keys(&self) -> Result<(), TemplateEngineError> {
        for key in self.sources.provided.keys() {
            if !self.schemas.contains_key(key) {
                return Err(TemplateEngineError::UnknownParam(key.clone()));
            }
        }

        Ok(())
    }

    fn resolve_one(
        &self,
        name: &str,
        schema: &ParamSchema,
    ) -> Result<Option<Value>, TemplateEngineError> {
        if let Some(value) = self.sources.provided.get(name).cloned() {
            return validate_resolved_value(name, value, &schema.value_type).map(Some);
        }

        if let Some(default) = &schema.default {
            let value = self.resolve_default(name, default)?;
            return validate_resolved_value(name, value, &schema.value_type).map(Some);
        }

        if schema.required {
            return Err(TemplateEngineError::MissingParam(name.to_string()));
        }

        Ok(None)
    }

    fn resolve_default(
        &self,
        name: &str,
        default: &ParamDefault,
    ) -> Result<Value, TemplateEngineError> {
        match default {
            ParamDefault::Literal(value) => Ok(value.clone()),
            ParamDefault::Source(source) => self.resolve_default_source(name, source),
        }
    }

    fn resolve_default_source(
        &self,
        name: &str,
        source: &ParamDefaultSource,
    ) -> Result<Value, TemplateEngineError> {
        let (source_name, key, resolved) = match source {
            ParamDefaultSource::Project(key) => {
                ("project", key, self.sources.project.resolve_param(key))
            }
            ParamDefaultSource::Context(key) => {
                ("context", key, self.sources.context.resolve_param(key))
            }
            ParamDefaultSource::Builtin(key) => {
                ("builtin", key, self.sources.builtin.resolve_param(key))
            }
        };

        match resolved {
            SourceValue::Found(value) => Ok(value),
            SourceValue::Missing => Err(TemplateEngineError::MissingParam(format!(
                "{source_name}.{key}"
            ))),
            SourceValue::Unknown => Err(TemplateEngineError::UnknownDefaultSource {
                param: name.to_string(),
                source_name: format!("{source_name}.{key}"),
            }),
        }
    }
}

fn validate_resolved_value(
    name: &str,
    value: Value,
    value_type: &ValueType,
) -> Result<Value, TemplateEngineError> {
    if value_matches_type(&value, value_type) {
        return Ok(value);
    }

    Err(TemplateEngineError::InvalidParam {
        name: name.to_string(),
        expected: value_type_name(value_type),
    })
}

fn value_matches_type(value: &Value, value_type: &ValueType) -> bool {
    match (value, value_type) {
        (Value::String(_), ValueType::String) => true,
        (Value::Integer(_), ValueType::Integer) => true,
        (Value::Bool(_), ValueType::Bool) => true,
        (Value::Date(_), ValueType::Date) => true,
        (Value::Array(values), ValueType::Array(item_type)) => values
            .iter()
            .all(|value| value_matches_type(value, item_type)),
        _ => false,
    }
}

fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::String => "string".to_string(),
        ValueType::Integer => "integer".to_string(),
        ValueType::Bool => "bool".to_string(),
        ValueType::Date => "date".to_string(),
        ValueType::Array(item_type) => format!("array<{}>", value_type_name(item_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceSchemaError, Span, validate_sources_schema, validate_sources_with_params};
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn schema(value_type: ValueType) -> ParamSchema {
        ParamSchema {
            value_type,
            required: true,
            default: None,
        }
    }

    fn resolver<'a>(
        schemas: &'a ParamSchemas,
        provided: &'a Params,
        project: &'a Params,
        context: &'a InitContext,
        builtin: &'a BuiltinContext,
    ) -> ParamResolver<'a> {
        ParamResolver {
            schemas,
            sources: ParamSources {
                provided,
                project,
                context,
                builtin,
            },
        }
    }

    #[test]
    fn resolves_provided_values_first() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "author".to_string(),
            ParamSchema {
                value_type: ValueType::String,
                required: true,
                default: Some(ParamDefault::Source(ParamDefaultSource::Project(
                    "author".to_string(),
                ))),
            },
        );
        let mut provided = Params::new();
        provided.insert("author".to_string(), Value::String("Provided".to_string()));
        let mut project = Params::new();
        project.insert("author".to_string(), Value::String("Project".to_string()));
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();

        assert_eq!(
            resolved.get("author"),
            Some(&Value::String("Provided".to_string()))
        );
    }

    #[test]
    fn resolves_project_default() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "author".to_string(),
            ParamSchema {
                value_type: ValueType::String,
                required: true,
                default: Some(ParamDefault::Source(ParamDefaultSource::Project(
                    "author".to_string(),
                ))),
            },
        );
        let provided = Params::new();
        let mut project = Params::new();
        project.insert("author".to_string(), Value::String("Sota".to_string()));
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();

        assert_eq!(
            resolved.get("author"),
            Some(&Value::String("Sota".to_string()))
        );
    }

    #[test]
    fn resolves_context_default() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "date".to_string(),
            ParamSchema {
                value_type: ValueType::Date,
                required: true,
                default: Some(ParamDefault::Source(ParamDefaultSource::Context(
                    "date".to_string(),
                ))),
            },
        );
        let provided = Params::new();
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();

        assert_eq!(resolved.get("date"), Some(&Value::Date(date(2026, 5, 22))));
    }

    #[test]
    fn resolves_builtin_default() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "date".to_string(),
            ParamSchema {
                value_type: ValueType::Date,
                required: true,
                default: Some(ParamDefault::Source(ParamDefaultSource::Builtin(
                    "today".to_string(),
                ))),
            },
        );
        let provided = Params::new();
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 1),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();

        assert_eq!(resolved.get("date"), Some(&Value::Date(date(2026, 5, 22))));
    }

    #[test]
    fn resolves_literal_default() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "title".to_string(),
            ParamSchema {
                value_type: ValueType::String,
                required: true,
                default: Some(ParamDefault::Literal(Value::String("Untitled".to_string()))),
            },
        );
        let provided = Params::new();
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();

        assert_eq!(
            resolved.get("title"),
            Some(&Value::String("Untitled".to_string()))
        );
    }

    #[test]
    fn omits_optional_value_without_default() {
        let mut schemas = ParamSchemas::new();
        schemas.insert(
            "subtitle".to_string(),
            ParamSchema {
                value_type: ValueType::String,
                required: false,
                default: None,
            },
        );
        let provided = Params::new();
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();

        assert!(!resolved.contains_key("subtitle"));

        let sources = [
            ("main.typ", "= Report"),
            ("parts/front.typ", "= {{ subtitle }}"),
        ];
        assert!(
            validate_sources_schema(sources, &schemas)
                .unwrap()
                .is_valid()
        );
        assert_eq!(
            validate_sources_with_params(sources, &schemas, &resolved)
                .unwrap()
                .errors,
            vec![SourceSchemaError::MissingInput {
                name: "subtitle".to_string(),
                label: Some("parts/front.typ".to_string()),
                span: Span { start: 2, end: 16 },
            }]
        );
        assert!(
            validate_sources_with_params([sources[0]], &schemas, &resolved)
                .unwrap()
                .is_valid()
        );

        let provided = toml::from_str(r#"subtitle = "Summary""#).unwrap();
        let resolved = resolver(&schemas, &provided, &project, &context, &builtin)
            .resolve()
            .unwrap();
        let validation = validate_sources_with_params(sources, &schemas, &resolved).unwrap();
        assert!(validation.is_valid());
        assert!(validation.is_warning_free());
    }

    #[test]
    fn rejects_missing_required_value() {
        let mut schemas = ParamSchemas::new();
        schemas.insert("title".to_string(), schema(ValueType::String));
        let provided = Params::new();
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        assert!(matches!(
            resolver(&schemas, &provided, &project, &context, &builtin)
                .resolve()
                .unwrap_err(),
            TemplateEngineError::MissingParam(name) if name == "title"
        ));
    }

    #[test]
    fn rejects_unknown_provided_value() {
        let schemas = ParamSchemas::new();
        let mut provided = Params::new();
        provided.insert("title".to_string(), Value::String("Report".to_string()));
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        assert!(matches!(
            resolver(&schemas, &provided, &project, &context, &builtin)
                .resolve()
                .unwrap_err(),
            TemplateEngineError::UnknownParam(name) if name == "title"
        ));
    }

    #[test]
    fn rejects_invalid_value_type() {
        let mut schemas = ParamSchemas::new();
        schemas.insert("count".to_string(), schema(ValueType::Integer));
        let mut provided = Params::new();
        provided.insert("count".to_string(), Value::String("3".to_string()));
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        assert!(matches!(
            resolver(&schemas, &provided, &project, &context, &builtin)
                .resolve()
                .unwrap_err(),
            TemplateEngineError::InvalidParam { name, expected }
            if name == "count" && expected == "integer"
        ));
    }

    #[test]
    fn validates_array_item_types_recursively() {
        let schemas = toml::from_str(
            r#"
[authors]
type = { array = "string" }

[dates]
type = { array = { array = "date" } }
"#,
        )
        .unwrap();
        let project = Params::new();
        let context = InitContext {
            template: None,
            date: date(2026, 5, 22),
        };
        let builtin = BuiltinContext {
            today: date(2026, 5, 22),
        };

        let mut provided = toml::from_str::<Params>(
            r#"
authors = ["Sota", "ChatGPT"]
dates = [[2026-05-22], [], [2026-05-23, 2026-05-24]]
"#,
        )
        .unwrap();
        assert_eq!(
            resolver(&schemas, &provided, &project, &context, &builtin)
                .resolve()
                .unwrap(),
            provided
        );

        for input in [
            r#"dates = [[2026-05-22], [2026-05-23, "2026-05-24"]]"#,
            "dates = [[2026-05-22], 2026-05-23]",
        ] {
            provided.extend(toml::from_str::<Params>(input).unwrap().into_inner());
            assert!(matches!(
                resolver(&schemas, &provided, &project, &context, &builtin).resolve(),
                Err(TemplateEngineError::InvalidParam { name, expected })
                    if name == "dates" && expected == "array<array<date>>"
            ));
        }
    }
}
