use std::collections::BTreeSet;

use super::ast::{NodeKind, Span};
use super::error::TemplateEngineError;
use super::parse::parse_template;
use super::schema::ParamSchemas;
use super::value::Params;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSchemaValidation {
    pub errors: Vec<SourceSchemaError>,
    pub warnings: Vec<SourceSchemaWarning>,
}

impl SourceSchemaValidation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn is_warning_free(&self) -> bool {
        self.warnings.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSchemaError {
    UndefinedInput {
        name: String,
        label: Option<String>,
        span: Span,
    },
    MissingInput {
        name: String,
        label: Option<String>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSchemaWarning {
    UnusedInput { name: String },
}

pub fn validate_source_schema(
    source: &str,
    param_schemas: &ParamSchemas,
) -> Result<SourceSchemaValidation, TemplateEngineError> {
    validate_sources([(None, source)], param_schemas, None)
}

/// Validates `(label, text)` pairs and reports unused inputs across the entire set.
pub fn validate_sources_schema<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
    param_schemas: &ParamSchemas,
) -> Result<SourceSchemaValidation, TemplateEngineError> {
    validate_sources(
        sources.into_iter().map(|(label, text)| (Some(label), text)),
        param_schemas,
        None,
    )
}

/// Validates labeled sources against the schema and the parameters after resolution.
/// A reference to a declared input with no resolved value is a `MissingInput` error.
pub fn validate_sources_with_params<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
    param_schemas: &ParamSchemas,
    resolved: &Params,
) -> Result<SourceSchemaValidation, TemplateEngineError> {
    validate_sources(
        sources.into_iter().map(|(label, text)| (Some(label), text)),
        param_schemas,
        Some(resolved),
    )
}

fn validate_sources<'a>(
    sources: impl IntoIterator<Item = (Option<&'a str>, &'a str)>,
    param_schemas: &ParamSchemas,
    resolved: Option<&Params>,
) -> Result<SourceSchemaValidation, TemplateEngineError> {
    let mut used_inputs = BTreeSet::new();
    let mut errors = Vec::new();

    for (label, text) in sources {
        let ast = parse_template(text).map_err(|error| match label {
            Some(label) => TemplateEngineError::ParseInSource {
                label: label.to_string(),
                error: Box::new(error),
            },
            None => error,
        })?;

        for node in ast.nodes {
            let (NodeKind::Display(param) | NodeKind::Literal(param)) = node.kind else {
                continue;
            };
            let name = param.name;
            if !param_schemas.contains_key(&name) {
                errors.push(SourceSchemaError::UndefinedInput {
                    name,
                    label: label.map(str::to_string),
                    span: node.span,
                });
            } else {
                if resolved.is_some_and(|params| !params.contains_key(&name)) {
                    errors.push(SourceSchemaError::MissingInput {
                        name: name.clone(),
                        label: label.map(str::to_string),
                        span: node.span,
                    });
                }
                used_inputs.insert(name);
            }
        }
    }

    let warnings = param_schemas
        .keys()
        .filter(|name| !used_inputs.contains(*name))
        .map(|name| SourceSchemaWarning::UnusedInput { name: name.clone() })
        .collect();

    Ok(SourceSchemaValidation { errors, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParamSchema, ParseErrorKind, ValueType};
    use std::collections::BTreeMap;

    fn schema(value_type: ValueType) -> ParamSchema {
        ParamSchema {
            value_type,
            required: true,
            default: None,
        }
    }

    #[test]
    fn validates_source_against_schema() {
        let mut schemas = ParamSchemas::new();
        schemas.insert("title".to_string(), schema(ValueType::String));
        schemas.insert(
            "authors".to_string(),
            schema(ValueType::Array(Box::new(ValueType::String))),
        );

        let validation =
            validate_source_schema("= {{ title }}\n#let authors = {{= authors }}", &schemas)
                .unwrap();

        assert!(validation.is_valid());
        assert!(validation.is_warning_free());
    }

    #[test]
    fn reports_undefined_inputs_with_span() {
        let schemas = ParamSchemas::new();

        let validation = validate_source_schema("= {{ title }}", &schemas).unwrap();

        assert_eq!(
            validation.errors,
            vec![SourceSchemaError::UndefinedInput {
                name: "title".to_string(),
                label: None,
                span: Span { start: 2, end: 13 },
            }]
        );
    }

    #[test]
    fn reports_unused_inputs() {
        let mut schemas = ParamSchemas::new();
        schemas.insert("title".to_string(), schema(ValueType::String));
        schemas.insert("unused".to_string(), schema(ValueType::String));

        let validation = validate_source_schema("= {{ title }}", &schemas).unwrap();

        assert_eq!(
            validation.warnings,
            vec![SourceSchemaWarning::UnusedInput {
                name: "unused".to_string(),
            }]
        );
    }

    #[test]
    fn returns_parse_error_for_invalid_source() {
        assert!(validate_source_schema("{{ title", &ParamSchemas::new()).is_err());
    }

    #[test]
    fn validates_references_and_unused_inputs_across_all_sources() {
        let schemas = ParamSchemas::from(BTreeMap::from([
            ("title".to_string(), schema(ValueType::String)),
            ("author".to_string(), schema(ValueType::String)),
            ("unused".to_string(), schema(ValueType::String)),
        ]));
        let sources = [
            ("main.typ", "{{ title }} {{ unknown }}"),
            ("parts/author.typ", "著者 {{= unknown }} {{ author }}"),
        ];

        let validation = validate_sources_schema(sources, &schemas).unwrap();

        assert_eq!(
            validation.errors,
            vec![
                SourceSchemaError::UndefinedInput {
                    name: "unknown".to_string(),
                    label: Some("main.typ".to_string()),
                    span: Span { start: 12, end: 25 },
                },
                SourceSchemaError::UndefinedInput {
                    name: "unknown".to_string(),
                    label: Some("parts/author.typ".to_string()),
                    span: Span { start: 7, end: 21 },
                },
            ]
        );
        assert_eq!(
            validation.warnings,
            vec![SourceSchemaWarning::UnusedInput {
                name: "unused".to_string(),
            }]
        );
    }

    #[test]
    fn labels_multi_source_parse_errors() {
        let error = validate_sources_schema(
            [("main.typ", "= Report"), ("broken.typ", "x {{")],
            &ParamSchemas::new(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            TemplateEngineError::ParseInSource { label, error }
                if label == "broken.typ" && matches!(
                    *error,
                    TemplateEngineError::Parse {
                        kind: ParseErrorKind::UnclosedTag,
                        span: Span { start: 2, end: 4 },
                    }
                )
        ));
    }
}
