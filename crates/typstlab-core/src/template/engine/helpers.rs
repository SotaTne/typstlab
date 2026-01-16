//! Helper functions for template rendering

use crate::template::error::TemplateError;
use toml::Value;

use super::TemplateContext;

/// Resolve a nested key from TOML data
pub(crate) fn resolve_key<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = data;

    for part in parts {
        current = match current {
            Value::Table(table) => table.get(part)?,
            _ => return None,
        };
    }

    Some(current)
}

/// Stringify a TOML value for template output
pub(crate) fn stringify_value(value: &Value, key: &str) -> Result<String, TemplateError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        Value::Datetime(dt) => Ok(dt.to_string()),
        Value::Array(_) => Err(TemplateError::ArrayInNonEachContext {
            key: key.to_string(),
        }),
        Value::Table(_) => Err(TemplateError::TableInPlaceholder {
            key: key.to_string(),
        }),
    }
}

/// Create a loop context with a variable binding
pub(crate) fn create_loop_context(
    base_data: &Value,
    var_name: &str,
    item: Value,
) -> TemplateContext {
    let mut table = if let Value::Table(t) = base_data {
        t.clone()
    } else {
        toml::map::Map::new()
    };

    table.insert(var_name.to_string(), item);
    TemplateContext::new(Value::Table(table))
}
