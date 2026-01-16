//! Shared test helpers for template engine tests

use crate::template::engine::TemplateContext;
use toml::{toml, Value};

/// Create a simple test context with basic scalar values
pub(super) fn simple_context() -> TemplateContext {
    let data = toml! {
        title = "My Title"
        count = 42
        price = 9.99
        enabled = true
        date = 2026-01-15
    };
    TemplateContext::new(Value::Table(data))
}

/// Create a nested test context with arrays and tables
pub(super) fn nested_context() -> TemplateContext {
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
