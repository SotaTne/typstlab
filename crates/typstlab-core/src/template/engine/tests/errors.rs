//! Error handling tests for template engine

use super::helpers::{nested_context, simple_context};
use super::*;

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
