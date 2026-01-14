//! Template error types

use std::fmt;

/// Template rendering errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    /// Key not found in data context
    UndefinedKey {
        /// The key that was not found
        key: String,
        /// Line number where the error occurred
        line: usize,
    },

    /// Malformed template syntax
    MalformedSyntax {
        /// Error message
        message: String,
        /// Line number where the error occurred
        line: usize,
    },

    /// Array used outside of {{each}} context
    ArrayInNonEachContext {
        /// The key that resolved to an array
        key: String,
    },

    /// Table used in placeholder (must use nested keys)
    TableInPlaceholder {
        /// The key that resolved to a table
        key: String,
    },
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::UndefinedKey { key, line } => {
                write!(f, "Undefined key '{}' at line {}", key, line)
            }
            TemplateError::MalformedSyntax { message, line } => {
                write!(f, "Malformed syntax at line {}: {}", line, message)
            }
            TemplateError::ArrayInNonEachContext { key } => {
                write!(
                    f,
                    "Array '{}' used outside of {{{{each}}}} context. Use {{{{each {} |item|}}}} ... {{{{/each}}}}",
                    key, key
                )
            }
            TemplateError::TableInPlaceholder { key } => {
                write!(
                    f,
                    "Table '{}' cannot be used directly in placeholder. Use nested keys like {}.field",
                    key, key
                )
            }
        }
    }
}

impl std::error::Error for TemplateError {}
