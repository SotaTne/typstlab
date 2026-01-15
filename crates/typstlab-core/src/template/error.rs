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

    /// Template rendering timed out (malformed input protection)
    Timeout {
        /// Maximum allowed duration
        max_duration: std::time::Duration,
        /// Actual elapsed time
        elapsed: std::time::Duration,
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
            TemplateError::Timeout {
                max_duration,
                elapsed,
            } => {
                write!(
                    f,
                    "Template rendering timed out after {:.2}s (max: {:.2}s). Check for unclosed {{{{...}}}} or {{{{each}}}} blocks.",
                    elapsed.as_secs_f64(),
                    max_duration.as_secs_f64()
                )
            }
        }
    }
}

impl std::error::Error for TemplateError {}
