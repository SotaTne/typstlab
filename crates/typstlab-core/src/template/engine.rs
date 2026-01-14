//! Template engine implementation

use crate::template::error::TemplateError;
use toml::Value;

/// Template context holding TOML data for rendering
#[derive(Debug, Clone)]
pub struct TemplateContext {
    data: Value,
}

impl TemplateContext {
    /// Create a new template context from TOML value
    pub fn new(data: Value) -> Self {
        Self { data }
    }

    /// Get the underlying TOML value
    pub fn data(&self) -> &Value {
        &self.data
    }
}

/// Template engine for rendering templates with TOML data
pub struct TemplateEngine;

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self
    }

    /// Render a template with the given context
    pub fn render(
        &self,
        template: &str,
        context: &TemplateContext,
    ) -> Result<String, TemplateError> {
        let mut output = String::new();
        let mut line = 1;
        let mut pos = 0;

        while pos < template.len() {
            let remaining = &template[pos..];

            // Find next {{
            if let Some(placeholder_start) = remaining.find("{{") {
                // Count backslashes before {{
                let mut backslash_count = 0;
                let mut text_end = placeholder_start;
                while text_end > 0 && remaining.as_bytes()[text_end - 1] == b'\\' {
                    backslash_count += 1;
                    text_end -= 1;
                }

                // Output text before backslashes
                if text_end > 0 {
                    let text = &remaining[..text_end];
                    output.push_str(text);
                    line += text.chars().filter(|&c| c == '\n').count();
                }

                // Output half of the backslashes (integer division)
                for _ in 0..(backslash_count / 2) {
                    output.push('\\');
                }

                // If odd number of backslashes, escape the {{}}
                if backslash_count % 2 == 1 {
                    // Escape placeholder - find closing }} and output literal {{...}}
                    let search_start = text_end + backslash_count + 2;
                    if let Some(close) = remaining[search_start..].find("}}") {
                        output.push_str("{{");
                        output.push_str(&remaining[search_start..search_start + close]);
                        output.push_str("}}");
                        pos += search_start + close + 2;
                        continue;
                    }
                }

                // Even number of backslashes (or zero) - process {{}} normally
                pos += text_end + backslash_count;

                // Now process the placeholder at current position
                let close = template[pos + 2..].find("}}").ok_or_else(|| {
                    TemplateError::MalformedSyntax {
                        message: "Unclosed placeholder or each loop".to_string(),
                        line,
                    }
                })?;

                let expr = template[pos + 2..pos + 2 + close].trim();

                // Check if it's an each loop
                if let Some(rest) = expr.strip_prefix("each ") {
                    // Parse: each key |var|
                    let pipe_pos =
                        rest.find('|')
                            .ok_or_else(|| TemplateError::MalformedSyntax {
                                message: format!(
                                    "Invalid each syntax: expected |var| in '{}'",
                                    expr
                                ),
                                line,
                            })?;

                    let key = rest[..pipe_pos].trim();
                    let var_end = rest[pipe_pos + 1..].find('|').ok_or_else(|| {
                        TemplateError::MalformedSyntax {
                            message: format!("Invalid each syntax: unclosed |var| in '{}'", expr),
                            line,
                        }
                    })?;

                    let var_name = rest[pipe_pos + 1..pipe_pos + 1 + var_end].trim();

                    // Find matching {{/each}} or {{ /each }}
                    let search_text = &template[pos + 2 + close + 2..];
                    let (loop_end, each_end_len) = find_each_end(search_text).ok_or_else(|| {
                        TemplateError::MalformedSyntax {
                            message: format!("Unclosed each loop for key '{}'", key),
                            line,
                        }
                    })?;

                    let loop_body = &template[pos + 2 + close + 2..pos + 2 + close + 2 + loop_end];

                    // Resolve array value
                    let array = resolve_key(context.data(), key).ok_or_else(|| {
                        TemplateError::UndefinedKey {
                            key: key.to_string(),
                            line,
                        }
                    })?;

                    let items = array
                        .as_array()
                        .ok_or_else(|| TemplateError::MalformedSyntax {
                            message: format!("Key '{}' is not an array", key),
                            line,
                        })?;

                    // Render loop body for each item
                    for item in items {
                        let loop_context =
                            create_loop_context(context.data(), var_name, item.clone());
                        let rendered = self.render(loop_body, &loop_context)?;
                        output.push_str(&rendered);
                    }

                    // Skip past {{/each}}
                    pos += 2 + close + 2 + loop_end + each_end_len;
                    let skipped_text =
                        &template[pos - (2 + close + 2 + loop_end + each_end_len)..pos];
                    line += skipped_text.chars().filter(|&c| c == '\n').count();
                    continue;
                } else if expr.starts_with("/each") {
                    return Err(TemplateError::MalformedSyntax {
                        message: "Unexpected {{/each}} without matching {{each}}".to_string(),
                        line,
                    });
                } else {
                    // Regular placeholder
                    let value = resolve_key(context.data(), expr).ok_or_else(|| {
                        TemplateError::UndefinedKey {
                            key: expr.to_string(),
                            line,
                        }
                    })?;

                    let stringified = stringify_value(value, expr)?;
                    output.push_str(&stringified);

                    pos += 2 + close + 2;
                    continue;
                }
            } else {
                // No more {{ found, output remaining text
                output.push_str(&template[pos..]);
                break;
            }
        }

        Ok(output)
    }
}

/// Find matching {{/each}} considering nested loops, return (position, length)
fn find_each_end(text: &str) -> Option<(usize, usize)> {
    let mut pos = 0;
    let mut depth = 0;

    while pos < text.len() {
        if let Some(start) = text[pos..].find("{{") {
            pos += start;
            if let Some(close) = text[pos + 2..].find("}}") {
                let expr = text[pos + 2..pos + 2 + close].trim();
                if expr.starts_with("each ") {
                    // Found nested {{each}}, increase depth
                    depth += 1;
                    pos += 2 + close + 2;
                } else if expr == "/each" {
                    if depth == 0 {
                        // Found matching {{/each}}
                        return Some((pos, 2 + close + 2));
                    } else {
                        // This is closing a nested each, decrease depth
                        depth -= 1;
                        pos += 2 + close + 2;
                    }
                } else {
                    pos += 2 + close + 2;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
    None
}

/// Resolve a nested key from TOML data
fn resolve_key<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
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
fn stringify_value(value: &Value, key: &str) -> Result<String, TemplateError> {
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
fn create_loop_context(base_data: &Value, var_name: &str, item: Value) -> TemplateContext {
    let mut table = if let Value::Table(t) = base_data {
        t.clone()
    } else {
        toml::map::Map::new()
    };

    table.insert(var_name.to_string(), item);
    TemplateContext::new(Value::Table(table))
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to render a template
pub fn render(template: &str, context: &TemplateContext) -> Result<String, TemplateError> {
    TemplateEngine::new().render(template, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::toml;

    // Helper to create a simple context
    fn simple_context() -> TemplateContext {
        let data = toml! {
            title = "My Title"
            count = 42
            price = 9.99
            enabled = true
            date = 2026-01-15
        };
        TemplateContext::new(Value::Table(data))
    }

    // Helper to create nested context
    fn nested_context() -> TemplateContext {
        let data = toml! {
            [paper]
            title = "Research Paper"
            language = "en"
            date = "2026-01-15"

            [[paper.authors]]
            name = "John Doe"
            email = "john@example.com"
            affiliation = "University"

            [[paper.authors]]
            name = "Jane Smith"
            email = "jane@example.com"
            affiliation = "Institute"
        };
        TemplateContext::new(Value::Table(data))
    }

    #[test]
    fn test_render_simple_placeholder() {
        let context = simple_context();
        let template = "Title: {{title}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Title: My Title");
    }

    #[test]
    fn test_render_placeholder_with_spaces() {
        let context = simple_context();
        let template = "Title: {{ title }}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Title: My Title");
    }

    #[test]
    fn test_render_placeholder_with_many_spaces() {
        let context = simple_context();
        let template = "Title: {{  title  }}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Title: My Title");
    }

    #[test]
    fn test_render_nested_key() {
        let context = nested_context();
        let template = "Paper: {{paper.title}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Paper: Research Paper");
    }

    #[test]
    fn test_render_nested_key_with_spaces() {
        let context = nested_context();
        let template = "Paper: {{ paper.title }}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Paper: Research Paper");
    }

    #[test]
    fn test_render_integer_value() {
        let context = simple_context();
        let template = "Count: {{count}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Count: 42");
    }

    #[test]
    fn test_render_float_value() {
        let context = simple_context();
        let template = "Price: {{price}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Price: 9.99");
    }

    #[test]
    fn test_render_boolean_value() {
        let context = simple_context();
        let template = "Enabled: {{enabled}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Enabled: true");
    }

    #[test]
    fn test_render_date_value() {
        let context = simple_context();
        let template = "Date: {{date}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Date: 2026-01-15");
    }

    #[test]
    fn test_render_each_loop() {
        let context = nested_context();
        let template = r#"{{each paper.authors |author|}}
Author: {{author.name}}
{{/each}}"#;
        let result = render(template, &context).unwrap();
        assert!(result.contains("Author: John Doe"));
        assert!(result.contains("Author: Jane Smith"));
    }

    #[test]
    fn test_render_each_loop_with_spaces() {
        let context = nested_context();
        let template = r#"{{ each paper.authors |author| }}
Author: {{ author.name }}
{{ /each }}"#;
        let result = render(template, &context).unwrap();
        assert!(result.contains("Author: John Doe"));
        assert!(result.contains("Author: Jane Smith"));
    }

    #[test]
    fn test_render_inline_each() {
        let context = nested_context();
        let template = "Authors: {{each paper.authors |author|}}{{author.name}}, {{/each}}";
        let result = render(template, &context).unwrap();
        assert!(result.contains("John Doe"));
        assert!(result.contains("Jane Smith"));
    }

    #[test]
    fn test_render_inline_each_with_spaces() {
        let context = nested_context();
        let template = "Authors: {{ each paper.authors |author| }} {{ author.name }}, {{ /each }}";
        let result = render(template, &context).unwrap();
        assert!(result.contains("John Doe"));
        assert!(result.contains("Jane Smith"));
    }

    #[test]
    fn test_render_nested_each_loops() {
        let data = toml! {
            [[papers]]
            title = "Paper 1"
            [[papers.authors]]
            name = "Alice"
            [[papers.authors]]
            name = "Bob"

            [[papers]]
            title = "Paper 2"
            [[papers.authors]]
            name = "Charlie"
        };
        let context = TemplateContext::new(Value::Table(data));

        let template = r#"{{each papers |paper|}}
= {{paper.title}}
{{each paper.authors |author|}}
- {{author.name}}
{{/each}}
{{/each}}"#;

        let result = render(template, &context).unwrap();
        assert!(result.contains("Paper 1"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
        assert!(result.contains("Paper 2"));
        assert!(result.contains("Charlie"));
    }

    #[test]
    fn test_render_escape_sequences() {
        let context = simple_context();
        let template = r#"Literal: \{{title}}"#;
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Literal: {{title}}");
    }

    #[test]
    fn test_render_escape_with_spaces() {
        let context = simple_context();
        let template = r#"Literal: \{{ title }}"#;
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Literal: {{ title }}");
    }

    #[test]
    fn test_render_double_backslash_escape() {
        let context = simple_context();
        let template = r#"Backslash: \\{{title}}"#;
        let result = render(template, &context).unwrap();
        assert_eq!(result, r#"Backslash: \My Title"#);
    }

    #[test]
    fn test_render_multiple_placeholders() {
        let context = nested_context();
        let template = "{{paper.title}} ({{paper.language}}) - {{paper.date}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "Research Paper (en) - 2026-01-15");
    }

    #[test]
    fn test_error_undefined_key() {
        let context = simple_context();
        let template = "Value: {{nonexistent}}";
        let result = render(template, &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::UndefinedKey { key, line }) => {
                assert_eq!(key, "nonexistent");
                assert_eq!(line, 1);
            }
            _ => panic!("Expected UndefinedKey error"),
        }
    }

    #[test]
    fn test_error_undefined_nested_key() {
        let context = nested_context();
        let template = "Value: {{paper.nonexistent}}";
        let result = render(template, &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::UndefinedKey { key, .. }) => {
                assert_eq!(key, "paper.nonexistent");
            }
            _ => panic!("Expected UndefinedKey error"),
        }
    }

    #[test]
    fn test_error_array_in_non_each() {
        let context = nested_context();
        let template = "Authors: {{paper.authors}}";
        let result = render(template, &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::ArrayInNonEachContext { key }) => {
                assert_eq!(key, "paper.authors");
            }
            _ => panic!("Expected ArrayInNonEachContext error"),
        }
    }

    #[test]
    fn test_error_table_in_placeholder() {
        let context = nested_context();
        let template = "Paper: {{paper}}";
        let result = render(template, &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::TableInPlaceholder { key }) => {
                assert_eq!(key, "paper");
            }
            _ => panic!("Expected TableInPlaceholder error"),
        }
    }

    #[test]
    fn test_error_malformed_unclosed_placeholder() {
        let context = simple_context();
        let template = "Value: {{title";
        let result = render(template, &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::MalformedSyntax { .. }) => {}
            _ => panic!("Expected MalformedSyntax error"),
        }
    }

    #[test]
    fn test_error_malformed_unclosed_each() {
        let context = nested_context();
        let template = "{{each paper.authors |author|}}{{author.name}}";
        let result = render(template, &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::MalformedSyntax { message, .. }) => {
                assert!(message.contains("Unclosed") || message.contains("each"));
            }
            _ => panic!("Expected MalformedSyntax error"),
        }
    }

    #[test]
    fn test_render_empty_array() {
        let data = toml! {
            items = []
        };
        let context = TemplateContext::new(Value::Table(data));
        let template = "{{each items |item|}}{{item}}{{/each}}";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_no_placeholders() {
        let context = simple_context();
        let template = "This is plain text with no placeholders.";
        let result = render(template, &context).unwrap();
        assert_eq!(result, "This is plain text with no placeholders.");
    }
}
