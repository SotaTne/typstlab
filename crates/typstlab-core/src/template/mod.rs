//! Template module - Pure text substitution template engine
//!
//! This module provides a lightweight template engine for generating Typst files
//! from `.tmp.typ` template files using TOML data context.
//!
//! ## Philosophy
//!
//! - **ERB/C macro style**: Pure text substitution, no Typst evaluation
//! - **`.tmp.typ` extension**: Template files are valid Typst before generation (IDE support)
//! - **Template author responsibility**: Template must generate valid Typst code
//! - **TOML data mapping**: All TOML-representable data can be used
//!
//! ## Syntax
//!
//! - Basic placeholders: `{{key}}` or `{{ key }}` (spaces optional)
//! - Nested access: `{{nested.key}}` or `{{ nested.key }}`
//! - List iteration: `{{each items |item|}} ... {{/each}}` or `{{ each items |item| }}`
//! - Escape sequences: `\{{literal}}` or `\{{ literal }}`

pub mod engine;
pub mod error;

pub use engine::{render, TemplateContext, TemplateEngine};
pub use error::TemplateError;
