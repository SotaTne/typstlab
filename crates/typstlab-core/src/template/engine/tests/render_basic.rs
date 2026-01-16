//! Basic rendering tests for template engine

use super::helpers::{nested_context, simple_context};
use super::*;

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
fn test_render_multiple_placeholders() {
    let context = nested_context();
    let template = "{{paper.title}} ({{paper.language}}) - {{paper.date}}";
    let result = render(template, &context).unwrap();
    assert_eq!(result, "Research Paper (en) - 2026-01-15");
}

#[test]
fn test_render_no_placeholders() {
    let context = simple_context();
    let template = "This is plain text with no placeholders.";
    let result = render(template, &context).unwrap();
    assert_eq!(result, "This is plain text with no placeholders.");
}
