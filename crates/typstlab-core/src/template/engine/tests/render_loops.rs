//! Loop rendering tests for template engine

use super::helpers::nested_context;
use super::*;
use toml::{toml, Value};

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
fn test_render_empty_array() {
    let data = toml! {
        items = []
    };
    let context = TemplateContext::new(Value::Table(data));
    let template = "{{each items |item|}}{{item}}{{/each}}";
    let result = render(template, &context).unwrap();
    assert_eq!(result, "");
}
