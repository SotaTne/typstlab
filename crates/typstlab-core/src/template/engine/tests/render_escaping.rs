//! Escape sequence tests for template engine

use super::helpers::simple_context;
use super::*;
use toml::{toml, Value};

#[test]
fn test_render_escape_sequences() {
    let context = simple_context();
    let template = r#"Literal: \{{title}}"#;
    let result = render(template, &context).unwrap();
    assert_eq!(result, "Literal: {{title}}");
}

#[test]
fn test_error_escaped_placeholder_unclosed() {
    // Regression test: escaped placeholder without closing }} should error
    let context = simple_context();
    let template = r#"Before \{{title after"#;
    let result = render(template, &context);
    assert!(
        result.is_err(),
        "Escaped placeholder without }} should error"
    );
    match result {
        Err(TemplateError::MalformedSyntax { message, .. }) => {
            assert!(
                message.contains("Unclosed"),
                "Error should mention unclosed placeholder"
            );
        }
        _ => panic!("Expected MalformedSyntax error for unclosed escaped placeholder"),
    }
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
fn test_render_escaped_each_in_loop_body() {
    // \{{each nested}} inside loop body should be treated as literal
    let data = toml! {
        [[items]]
        name = "Item1"
        [[items]]
        name = "Item2"
    };
    let context = TemplateContext::new(Value::Table(data));
    let template = r#"{{each items |item|}}{{item.name}}: \{{each nested}}
{{/each}}"#;
    let result = render(template, &context).unwrap();
    assert!(result.contains("Item1: {{each nested}}"));
    assert!(result.contains("Item2: {{each nested}}"));
}

#[test]
fn test_render_escaped_end_each_in_loop_body() {
    // \{{/each}} inside loop body should be treated as literal, not loop closing
    let data = toml! {
        [[items]]
        name = "Item1"
        [[items]]
        name = "Item2"
    };
    let context = TemplateContext::new(Value::Table(data));
    let template = r#"{{each items |item|}}{{item.name}}: \{{/each}} more content
{{/each}}"#;
    let result = render(template, &context).unwrap();
    // \{{/each}} should appear as literal in output
    assert!(result.contains("Item1: {{/each}} more content"));
    assert!(result.contains("Item2: {{/each}} more content"));
    // And the loop should have closed properly (both items rendered)
    let line_count = result.lines().count();
    assert_eq!(line_count, 2, "Loop should render for both items");
}

#[test]
fn test_render_triple_backslash_each() {
    // \\\{{each}} → \ + {{each}} (backslash + literal)
    let data = toml! {
        [[items]]
        name = "Item1"
    };
    let context = TemplateContext::new(Value::Table(data));
    let template = r#"{{each items |item|}}\\\{{each nested}}{{/each}}"#;
    let result = render(template, &context).unwrap();
    // Should output: \ + {{each nested}}
    assert_eq!(result, r#"\{{each nested}}"#);
}

#[test]
fn test_render_quadruple_backslash_each() {
    // \\\\{{each}} → \\ + start nested each (should error: undefined key)
    let data = toml! {
        [[items]]
        name = "Item1"
        [[items.nested]]
        value = "Nested1"
    };
    let context = TemplateContext::new(Value::Table(data));
    let template =
        r#"{{each items |item|}}\\\\{{each item.nested |n|}}{{n.value}}{{/each}}{{/each}}"#;
    let result = render(template, &context).unwrap();
    // Should output: \\ + nested loop result
    assert!(result.contains(r#"\\Nested1"#));
}

#[test]
fn test_render_escaped_end_each_with_spaces() {
    // \{{ /each }} with spaces should also be treated as literal
    let data = toml! {
        [[items]]
        name = "Item1"
    };
    let context = TemplateContext::new(Value::Table(data));
    let template = r#"{{each items |item|}}{{item.name}}: \{{ /each }}
{{/each}}"#;
    let result = render(template, &context).unwrap();
    assert!(result.contains("Item1: {{ /each }}"));
}
