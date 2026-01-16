//! Template engine implementation

mod blocks;
mod helpers;
mod tokenize;

use crate::template::error::TemplateError;
use std::time::{Duration, Instant};
use toml::Value;

use blocks::find_each_end;
use helpers::{create_loop_context, resolve_key, stringify_value};

/// Maximum duration for template rendering (malformed input protection)
const RENDER_TIMEOUT: Duration = Duration::from_secs(10);

/// Check if rendering has exceeded the timeout
fn check_timeout(start: Instant) -> Result<(), TemplateError> {
    let elapsed = start.elapsed();
    if elapsed >= RENDER_TIMEOUT {
        return Err(TemplateError::Timeout {
            max_duration: RENDER_TIMEOUT,
            elapsed,
        });
    }
    Ok(())
}

/// Process text before placeholder and handle backslashes
///
/// Returns (position_offset, new_line_number)
fn process_text_and_backslashes(
    remaining: &str,
    placeholder_start: usize,
    output: &mut String,
    line: usize,
) -> (usize, usize) {
    let backslash_count = count_backslashes_before(remaining, placeholder_start);
    let text_end = placeholder_start - backslash_count;

    // Output text before backslashes
    let mut new_line = line;
    if text_end > 0 {
        let text = &remaining[..text_end];
        output.push_str(text);
        new_line += count_newlines(text);
    }

    // Output half of the backslashes (integer division)
    for _ in 0..(backslash_count / 2) {
        output.push('\\');
    }

    (text_end + backslash_count, new_line)
}

/// Count backslashes immediately before a position
fn count_backslashes_before(text: &str, pos: usize) -> usize {
    let mut count = 0;
    let mut check_pos = pos;
    while check_pos > 0 && text.as_bytes()[check_pos - 1] == b'\\' {
        count += 1;
        check_pos -= 1;
    }
    count
}

/// Check if a placeholder is escaped (odd number of backslashes)
fn is_escaped_placeholder(remaining: &str, placeholder_start: usize) -> bool {
    let backslash_count = count_backslashes_before(remaining, placeholder_start);
    backslash_count % 2 == 1
}

/// Handle an escaped placeholder by outputting it literally
///
/// Returns number of bytes to skip
fn handle_escaped_placeholder(
    remaining: &str,
    placeholder_start: usize,
    output: &mut String,
    line: usize,
) -> Result<usize, TemplateError> {
    let backslash_count = count_backslashes_before(remaining, placeholder_start);
    let text_end = placeholder_start - backslash_count;
    let search_start = text_end + backslash_count + 2;

    if let Some(close) = remaining[search_start..].find("}}") {
        output.push_str("{{");
        output.push_str(&remaining[search_start..search_start + close]);
        output.push_str("}}");
        Ok(search_start + close + 2)
    } else {
        // Escaped placeholder without closing }} is malformed
        Err(TemplateError::MalformedSyntax {
            message: "Unclosed escaped placeholder".to_string(),
            line,
        })
    }
}

/// Find closing }} for a placeholder
fn find_closing_braces(template: &str, pos: usize, line: usize) -> Result<usize, TemplateError> {
    template[pos + 2..]
        .find("}}")
        .ok_or_else(|| TemplateError::MalformedSyntax {
            message: "Unclosed placeholder or each loop".to_string(),
            line,
        })
}

/// Parse each loop syntax: "items |item|" → (key, var_name)
fn parse_each_syntax(rest: &str, line: usize) -> Result<(&str, &str), TemplateError> {
    let pipe_pos = rest
        .find('|')
        .ok_or_else(|| TemplateError::MalformedSyntax {
            message: format!("Invalid each syntax: expected |var| in 'each {}'", rest),
            line,
        })?;

    let key = rest[..pipe_pos].trim();
    let var_end = rest[pipe_pos + 1..]
        .find('|')
        .ok_or_else(|| TemplateError::MalformedSyntax {
            message: format!("Invalid each syntax: unclosed |var| in 'each {}'", rest),
            line,
        })?;

    let var_name = rest[pipe_pos + 1..pipe_pos + 1 + var_end].trim();
    Ok((key, var_name))
}

/// Resolve an array value from context
fn resolve_array<'a>(
    data: &'a Value,
    key: &str,
    line: usize,
) -> Result<&'a Vec<Value>, TemplateError> {
    let array = resolve_key(data, key).ok_or_else(|| TemplateError::UndefinedKey {
        key: key.to_string(),
        line,
    })?;

    array
        .as_array()
        .ok_or_else(|| TemplateError::MalformedSyntax {
            message: format!("Key '{}' is not an array", key),
            line,
        })
}

/// Count newlines in text
fn count_newlines(text: &str) -> usize {
    text.chars().filter(|&c| c == '\n').count()
}

/// Rendering state passed between helper functions
struct RenderState<'a> {
    template: &'a str,
    remaining: &'a str,
    placeholder_start: usize,
    pos: usize,
    line: usize,
}

/// Loop parsing state for each loop processing
struct LoopState<'a> {
    template: &'a str,
    pos: usize,
    close: usize,
    rest: &'a str,
    line: usize,
}

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
        let start = Instant::now();
        let mut output = String::new();
        let mut line = 1;
        let mut pos = 0;

        while pos < template.len() {
            check_timeout(start)?;
            let remaining = &template[pos..];

            if let Some(placeholder_start) = remaining.find("{{") {
                let state = RenderState {
                    template,
                    remaining,
                    placeholder_start,
                    pos,
                    line,
                };
                let (skip, new_line) = self.process_placeholder(&state, context, &mut output)?;
                pos += skip;
                line = new_line;
            } else {
                output.push_str(&template[pos..]);
                break;
            }
        }

        Ok(output)
    }

    /// Process a single placeholder and return (bytes_to_skip, new_line_number)
    fn process_placeholder(
        &self,
        state: &RenderState,
        context: &TemplateContext,
        output: &mut String,
    ) -> Result<(usize, usize), TemplateError> {
        let (text_skip, new_line) = process_text_and_backslashes(
            state.remaining,
            state.placeholder_start,
            output,
            state.line,
        );

        // Check if placeholder is escaped
        if is_escaped_placeholder(state.remaining, state.placeholder_start) {
            let skip = handle_escaped_placeholder(
                state.remaining,
                state.placeholder_start,
                output,
                new_line,
            )?;
            return Ok((skip, new_line));
        }

        self.process_unescaped_placeholder(state, text_skip, new_line, context, output)
    }

    /// Process an unescaped placeholder ({{expr}}, {{each}}, etc.)
    fn process_unescaped_placeholder(
        &self,
        state: &RenderState,
        text_skip: usize,
        new_line: usize,
        context: &TemplateContext,
        output: &mut String,
    ) -> Result<(usize, usize), TemplateError> {
        let actual_pos = state.pos + text_skip;
        let close = find_closing_braces(state.template, actual_pos, new_line)?;
        let expr = state.template[actual_pos + 2..actual_pos + 2 + close].trim();

        if let Some(rest) = expr.strip_prefix("each ") {
            self.handle_each_loop(
                state, text_skip, actual_pos, close, rest, new_line, context, output,
            )
        } else if expr.starts_with("/each") {
            Err(TemplateError::MalformedSyntax {
                message: "Unexpected {{/each}} without matching {{each}}".to_string(),
                line: new_line,
            })
        } else {
            self.process_regular_placeholder(expr, context, output, new_line)?;
            Ok((text_skip + 2 + close + 2, new_line))
        }
    }

    /// Handle an each loop placeholder
    #[allow(clippy::too_many_arguments)]
    fn handle_each_loop(
        &self,
        state: &RenderState,
        text_skip: usize,
        actual_pos: usize,
        close: usize,
        rest: &str,
        new_line: usize,
        context: &TemplateContext,
        output: &mut String,
    ) -> Result<(usize, usize), TemplateError> {
        let loop_state = LoopState {
            template: state.template,
            pos: actual_pos,
            close,
            rest,
            line: new_line,
        };
        let skip = self.process_each_loop(&loop_state, context, output)?;
        let final_line = new_line + count_newlines(&state.template[actual_pos..actual_pos + skip]);
        Ok((text_skip + skip, final_line))
    }

    /// Process an each loop and return bytes to skip
    fn process_each_loop(
        &self,
        loop_state: &LoopState,
        context: &TemplateContext,
        output: &mut String,
    ) -> Result<usize, TemplateError> {
        let (key, var_name) = parse_each_syntax(loop_state.rest, loop_state.line)?;

        // Find matching {{/each}}
        let search_text = &loop_state.template[loop_state.pos + 2 + loop_state.close + 2..];
        let (loop_end, each_end_len) =
            find_each_end(search_text).ok_or_else(|| TemplateError::MalformedSyntax {
                message: format!("Unclosed each loop for key '{}'", key),
                line: loop_state.line,
            })?;

        let body_start = loop_state.pos + 2 + loop_state.close + 2;
        let loop_body = &loop_state.template[body_start..body_start + loop_end];

        // Resolve and render array items
        let items = resolve_array(context.data(), key, loop_state.line)?;
        for item in items {
            let loop_context = create_loop_context(context.data(), var_name, item.clone());
            let rendered = self.render(loop_body, &loop_context)?;
            output.push_str(&rendered);
        }

        Ok(2 + loop_state.close + 2 + loop_end + each_end_len)
    }

    /// Process a regular placeholder ({{key}})
    fn process_regular_placeholder(
        &self,
        expr: &str,
        context: &TemplateContext,
        output: &mut String,
        line: usize,
    ) -> Result<(), TemplateError> {
        let value =
            resolve_key(context.data(), expr).ok_or_else(|| TemplateError::UndefinedKey {
                key: expr.to_string(),
                line,
            })?;

        let stringified = stringify_value(value, expr)?;
        output.push_str(&stringified);
        Ok(())
    }
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
mod tests;
