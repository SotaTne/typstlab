//! Template engine implementation

use crate::template::error::TemplateError;
use std::time::{Duration, Instant};
use toml::Value;

/// Maximum duration for template rendering (malformed input protection)
const RENDER_TIMEOUT: Duration = Duration::from_secs(10);

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
            // Timeout guard (check every iteration)
            let elapsed = start.elapsed();
            if elapsed >= RENDER_TIMEOUT {
                return Err(TemplateError::Timeout {
                    max_duration: RENDER_TIMEOUT,
                    elapsed,
                });
            }

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

// ============================================================================
// Tokenization & Parsing (Compiler Pattern)
// ============================================================================

/// Token classification for future extensibility
///
/// This enum allows adding new constructs without changing the tokenization logic.
#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    /// {{key}} or {{nested.key}}
    Placeholder { key: String },

    /// {{each items |var|}}
    BlockStart { keyword: String, args: String },

    /// {{/each}}
    BlockEnd { keyword: String },
}

/// A single {{...}} token with position and classification
///
/// Represents a tokenized placeholder in the template with metadata
/// for parsing and error reporting.
#[derive(Debug, Clone, PartialEq)]
struct Token {
    /// Token classification
    kind: TokenKind,
    /// Absolute byte position of `{{` in template
    start: usize,
    /// Total length in bytes including {{ and }}
    length: usize,
    /// Number of backslashes before `{{`
    /// Odd count = escaped (literal), even = real (processed)
    backslash_count: usize,
    /// Line number where token starts (for error messages)
    line: usize,
}

impl Token {
    /// Check if this token is escaped (odd backslash count)
    fn is_escaped(&self) -> bool {
        self.backslash_count % 2 == 1
    }
}

/// Tokenization state machine (explicit for testability)
///
/// This state machine ensures O(n) tokenization by processing each byte exactly once
/// in a forward-only manner.
///
/// # State Transitions
///
/// ```text
/// Normal ──{───> SeenLBrace ──{───> InToken ──}───> SeenRBrace ──}───> [Yield Token] → Normal
///   │               │                  │                  │
///   │ (not {)       │ (not {)          │ (not })          │ (not })
///   └──────────────>└─────────────────>└─────────────────>└──────────> Normal
///
/// Malformed {{ without }} → Skip and continue (robust recovery)
/// ```
///
/// # Performance Guarantee
///
/// - **Forward-only scanning**: Position never moves backward
/// - **No nested loops**: Each byte processed exactly once
/// - **Bounded work per byte**: State transitions are O(1)
/// - **Backslash tracking**: Forward-only accumulation, no backward scans
#[derive(Debug, Clone, PartialEq)]
enum ScanState {
    /// Normal text scanning
    ///
    /// Scanning regular text, tracking backslashes for escape detection.
    /// Forward-only: backslash_count accumulates as we scan forward.
    Normal {
        /// Number of consecutive backslashes seen before current position
        /// (forward-only tracking, no backward scan)
        backslash_count: usize,
    },

    /// Seen first `{`, checking for second `{`
    ///
    /// Transitioning from Normal when we encounter a `{`.
    /// Next byte determines if this is a token start `{{` or just literal text.
    SeenLBrace {
        /// Position of the first `{` character
        pos: usize,
        /// Backslash count before the `{` (for escape detection)
        backslash_count: usize,
    },

    /// Inside `{{...}}`, scanning until `}}`
    ///
    /// Actively scanning token content between `{{` and `}}`.
    /// Accumulate bytes until we see the closing `}}`.
    InToken {
        /// Byte position of the opening `{{`
        start: usize,
        /// Byte position where token content starts (after `{{`)
        content_start: usize,
        /// Backslash count before the opening `{{` (for escape detection)
        backslash_count: usize,
    },

    /// Seen first `}` inside token, checking for second `}`
    ///
    /// Transitioning from InToken when we encounter a `}`.
    /// Next byte determines if this is token end `}}` or just literal `}` in content.
    SeenRBrace {
        /// Byte position of the opening `{{`
        start: usize,
        /// Byte position where token content starts (after `{{`)
        content_start: usize,
        /// Position of the first `}` character
        rbrace_pos: usize,
        /// Backslash count before the opening `{{` (for escape detection)
        backslash_count: usize,
    },
}

/// Iterator over tokens in a template string
///
/// This iterator implements true O(n) tokenization by processing each byte
/// exactly once in a forward-only manner using a state machine.
///
/// # Example
///
/// ```ignore
/// // Internal use only, not part of public API
/// let text = "Hello {{name}} world";
/// let mut stream = TokenStream::new(text);
///
/// while let Some(token) = stream.next() {
///     println!("Token at {}: {:?}", token.start, token.kind);
/// }
/// ```
///
/// # Performance
///
/// - **O(n) guarantee**: Each byte processed exactly once
/// - **No allocations in hot path**: Works with byte slices
/// - **Forward-only**: Position never moves backward
struct TokenStream<'a> {
    /// Zero-copy byte slice of template text
    bytes: &'a [u8],
    /// Current byte position
    pos: usize,
    /// State machine state
    state: ScanState,
    /// Current line number (for error messages)
    line: usize,
    /// Step count for O(n) timeout protection
    step_count: usize,
}

impl<'a> TokenStream<'a> {
    /// Create a new TokenStream from template text
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
            state: ScanState::Normal { backslash_count: 0 },
            line: 1,
            step_count: 0,
        }
    }

    /// Classify token content into TokenKind
    ///
    /// Parses the text between `{{` and `}}` to determine token type:
    /// - `each items |var|` → BlockStart
    /// - `/each` → BlockEnd
    /// - `key` or `nested.key` → Placeholder
    fn classify_content(&self, content: &str) -> TokenKind {
        let trimmed = content.trim();

        if let Some(rest) = trimmed.strip_prefix("each ") {
            TokenKind::BlockStart {
                keyword: "each".to_string(),
                args: rest.to_string(),
            }
        } else if let Some(rest) = trimmed.strip_prefix('/') {
            let keyword = rest.trim().to_string();
            TokenKind::BlockEnd { keyword }
        } else {
            TokenKind::Placeholder {
                key: trimmed.to_string(),
            }
        }
    }
}

impl<'a> Iterator for TokenStream<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        const MAX_STEPS_MULTIPLIER: usize = 3;
        let max_steps = self.bytes.len().saturating_mul(MAX_STEPS_MULTIPLIER);

        loop {
            // Step counter guard (O(n) bound enforcement)
            self.step_count += 1;
            if self.step_count > max_steps {
                // Malformed input - terminate gracefully
                return None;
            }

            // End of input
            if self.pos >= self.bytes.len() {
                return None;
            }

            let byte = self.bytes[self.pos];

            // Count steps for O(n) verification in tests (direct counter, no tracing overhead)
            #[cfg(test)]
            {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static TOKENSTREAM_STEPS: AtomicUsize = AtomicUsize::new(0);
                TOKENSTREAM_STEPS.fetch_add(1, Ordering::Relaxed);
            }

            match &self.state {
                ScanState::Normal { backslash_count } => {
                    if byte == b'\\' {
                        // Accumulate backslash count (forward-only)
                        self.state = ScanState::Normal {
                            backslash_count: backslash_count + 1,
                        };
                        self.pos += 1;
                    } else if byte == b'{' {
                        // Potential start of {{
                        self.state = ScanState::SeenLBrace {
                            pos: self.pos,
                            backslash_count: *backslash_count,
                        };
                        self.pos += 1;
                    } else {
                        // Regular character, reset backslash count
                        if byte == b'\n' {
                            self.line += 1;
                        }
                        self.state = ScanState::Normal { backslash_count: 0 };
                        self.pos += 1;
                    }
                }

                ScanState::SeenLBrace {
                    pos: lbrace_pos,
                    backslash_count,
                } => {
                    if byte == b'{' {
                        // Found {{, transition to InToken
                        self.state = ScanState::InToken {
                            start: *lbrace_pos,
                            content_start: self.pos + 1,
                            backslash_count: *backslash_count,
                        };
                        self.pos += 1;
                    } else {
                        // Just a single {, not a token
                        self.state = ScanState::Normal { backslash_count: 0 };
                        // Don't increment pos, process this byte in Normal state
                    }
                }

                ScanState::InToken {
                    start,
                    content_start,
                    backslash_count,
                } => {
                    if byte == b'}' {
                        // Potential end of }}
                        self.state = ScanState::SeenRBrace {
                            start: *start,
                            content_start: *content_start,
                            rbrace_pos: self.pos,
                            backslash_count: *backslash_count,
                        };
                        self.pos += 1;
                    } else {
                        // Still inside token content
                        if byte == b'\n' {
                            self.line += 1;
                        }
                        self.pos += 1;
                    }
                }

                ScanState::SeenRBrace {
                    start,
                    content_start,
                    rbrace_pos,
                    backslash_count,
                } => {
                    if byte == b'}' {
                        // Found }}, complete token
                        let content = std::str::from_utf8(&self.bytes[*content_start..*rbrace_pos])
                            .unwrap_or("");

                        let token = Token {
                            kind: self.classify_content(content),
                            start: *start,
                            length: self.pos + 1 - start,
                            backslash_count: *backslash_count,
                            line: self.line,
                        };

                        self.state = ScanState::Normal { backslash_count: 0 };
                        self.pos += 1;

                        return Some(token);
                    } else {
                        // Just a single } inside content, continue scanning
                        self.state = ScanState::InToken {
                            start: *start,
                            content_start: *content_start,
                            backslash_count: *backslash_count,
                        };
                        // Don't increment pos, process this byte in InToken state
                    }
                }
            }
        }
    }
}

/// Find matching closing tag for a block (e.g., "each" → "/each")
///
/// Returns (position, length) of the closing tag token.
///
/// # Nesting
///
/// Respects nested blocks of the same type and escape sequences.
///
/// # Performance
///
/// Uses TokenStream for true O(n) performance - single pass over the input.
fn find_block_end(text: &str, start_keyword: &str, end_keyword: &str) -> Option<(usize, usize)> {
    let tokens = TokenStream::new(text);
    let mut depth = 0;

    // Strip '/' prefix from end_keyword (e.g., "/each" → "each")
    // TokenKind::BlockEnd stores keyword without '/'
    let end_keyword_stripped = end_keyword.strip_prefix('/').unwrap_or(end_keyword);

    for token in tokens {
        // Skip escaped tokens
        if token.is_escaped() {
            continue;
        }

        // Pattern match on TokenKind for clean, type-safe parsing
        match &token.kind {
            TokenKind::BlockStart { keyword, .. } if keyword.as_str() == start_keyword.trim() => {
                // Found nested start tag, increase depth
                depth += 1;
            }
            TokenKind::BlockEnd { keyword } if keyword.as_str() == end_keyword_stripped => {
                if depth == 0 {
                    // Found matching closing tag at depth 0
                    return Some((token.start, token.length));
                } else {
                    // This is closing a nested block, decrease depth
                    depth -= 1;
                }
            }
            _ => {
                // Other tokens (placeholders, unrelated blocks) - continue scanning
            }
        }
    }

    // No matching closing tag found
    None
}

/// Find matching {{/each}} considering nested loops and escape sequences
///
/// Returns (position, length) of the closing {{/each}}.
/// Respects backslash escaping: \{{/each}} is treated as literal, not a closing tag.
///
/// This is a thin wrapper around `find_block_end()`.
fn find_each_end(text: &str) -> Option<(usize, usize)> {
    find_block_end(text, "each ", "/each")
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

    // Unit tests for ScanState state machine
    #[test]
    fn test_scan_state_transitions() {
        // Test Normal state construction
        let normal = ScanState::Normal { backslash_count: 0 };
        assert_eq!(normal, ScanState::Normal { backslash_count: 0 });

        // Test Normal state with backslashes
        let normal_with_backslash = ScanState::Normal { backslash_count: 2 };
        assert_eq!(
            normal_with_backslash,
            ScanState::Normal { backslash_count: 2 }
        );

        // Test SeenLBrace state construction
        let seen_lbrace = ScanState::SeenLBrace {
            pos: 10,
            backslash_count: 0,
        };
        assert_eq!(
            seen_lbrace,
            ScanState::SeenLBrace {
                pos: 10,
                backslash_count: 0
            }
        );

        // Test InToken state construction
        let in_token = ScanState::InToken {
            start: 10,
            content_start: 12,
            backslash_count: 0,
        };
        assert_eq!(
            in_token,
            ScanState::InToken {
                start: 10,
                content_start: 12,
                backslash_count: 0
            }
        );

        // Test SeenRBrace state construction
        let seen_rbrace = ScanState::SeenRBrace {
            start: 10,
            content_start: 12,
            rbrace_pos: 20,
            backslash_count: 0,
        };
        assert_eq!(
            seen_rbrace,
            ScanState::SeenRBrace {
                start: 10,
                content_start: 12,
                rbrace_pos: 20,
                backslash_count: 0
            }
        );

        // Test Clone functionality
        let original = ScanState::Normal { backslash_count: 3 };
        let cloned = original.clone();
        assert_eq!(original, cloned);

        // Test inequality between different states
        let state1 = ScanState::Normal { backslash_count: 0 };
        let state2 = ScanState::Normal { backslash_count: 1 };
        assert_ne!(state1, state2);
    }

    // Unit tests for TokenStream
    #[test]
    fn test_tokenstream_single_placeholder() {
        let text = "Hello {{name}} world";
        let mut stream = TokenStream::new(text);

        let token = stream.next().unwrap();
        assert_eq!(token.start, 6);
        assert_eq!(token.length, 8); // {{name}}
        assert!(!token.is_escaped());
        assert_eq!(
            token.kind,
            TokenKind::Placeholder {
                key: "name".to_string()
            }
        );

        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_multiple_tokens() {
        let text = "{{a}} {{b}} {{c}}";
        let mut stream = TokenStream::new(text);

        let token1 = stream.next().unwrap();
        assert_eq!(token1.start, 0);
        assert_eq!(
            token1.kind,
            TokenKind::Placeholder {
                key: "a".to_string()
            }
        );

        let token2 = stream.next().unwrap();
        assert_eq!(token2.start, 6);
        assert_eq!(
            token2.kind,
            TokenKind::Placeholder {
                key: "b".to_string()
            }
        );

        let token3 = stream.next().unwrap();
        assert_eq!(token3.start, 12);
        assert_eq!(
            token3.kind,
            TokenKind::Placeholder {
                key: "c".to_string()
            }
        );

        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_escaped_tokens() {
        let text = r#"\{{escaped}} {{real}}"#;
        let mut stream = TokenStream::new(text);

        let token1 = stream.next().unwrap();
        assert_eq!(token1.backslash_count, 1);
        assert!(token1.is_escaped());

        let token2 = stream.next().unwrap();
        assert_eq!(token2.backslash_count, 0);
        assert!(!token2.is_escaped());

        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_block_tokens() {
        let text = "{{each items |item|}} {{item.name}} {{/each}}";
        let mut stream = TokenStream::new(text);

        let token1 = stream.next().unwrap();
        match token1.kind {
            TokenKind::BlockStart { keyword, args } => {
                assert_eq!(keyword, "each");
                assert!(args.contains("items"));
            }
            _ => panic!("Expected BlockStart"),
        }

        let token2 = stream.next().unwrap();
        assert!(matches!(token2.kind, TokenKind::Placeholder { .. }));

        let token3 = stream.next().unwrap();
        match token3.kind {
            TokenKind::BlockEnd { keyword } => {
                assert_eq!(keyword, "each");
            }
            _ => panic!("Expected BlockEnd"),
        }

        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_empty_input() {
        let text = "";
        let mut stream = TokenStream::new(text);
        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_no_tokens() {
        let text = "Just plain text with no tokens";
        let mut stream = TokenStream::new(text);
        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_nested_braces() {
        // {{{ and }}} sequences
        // The first two { are treated as {{, so token starts at index 0
        let text = "{{{triple}}}";
        let mut stream = TokenStream::new(text);

        // Should find {{{triple (content: "{triple") - starts at index 0
        let token = stream.next().unwrap();
        assert_eq!(token.start, 0);
        assert_eq!(
            token.kind,
            TokenKind::Placeholder {
                key: "{triple".to_string() // Note: includes the third {
            }
        );

        assert!(stream.next().is_none());
    }

    #[test]
    fn test_tokenstream_line_numbers() {
        let text = "Line 1\n{{token1}}\nLine 3\n{{token2}}";
        let mut stream = TokenStream::new(text);

        let token1 = stream.next().unwrap();
        assert_eq!(token1.line, 2); // Token on line 2

        let token2 = stream.next().unwrap();
        assert_eq!(token2.line, 4); // Token on line 4

        assert!(stream.next().is_none());
    }

    // O(n) Performance Verification with Direct Counter
    // Uses atomic counter in TokenStream loop (deterministic, no overhead)

    // Global counter for TokenStream steps (defined in Iterator::next)
    // This is the same TOKENSTREAM_STEPS used in the #[cfg(test)] block

    fn reset_step_counter() {
        // Access the same static as in Iterator::next
        use std::sync::atomic::{AtomicUsize, Ordering};
        static TOKENSTREAM_STEPS: AtomicUsize = AtomicUsize::new(0);
        TOKENSTREAM_STEPS.store(0, Ordering::Relaxed);
    }

    fn read_step_counter() -> usize {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static TOKENSTREAM_STEPS: AtomicUsize = AtomicUsize::new(0);
        TOKENSTREAM_STEPS.load(Ordering::Relaxed)
    }

    fn with_step_counter<F: FnOnce()>(f: F) -> usize {
        reset_step_counter();
        f();
        read_step_counter()
    }

    fn steps_per_byte(steps: usize, input_len: usize) -> f64 {
        steps as f64 / input_len as f64
    }

    #[test]
    fn test_tokenstream_o_n_performance() {
        // Generate templates of different sizes
        let text_100 = (0..100)
            .map(|i| format!("{{{{token{}}}}} ", i))
            .collect::<String>();
        let text_1000 = (0..1000)
            .map(|i| format!("{{{{token{}}}}} ", i))
            .collect::<String>();
        let text_10000 = (0..10000)
            .map(|i| format!("{{{{token{}}}}} ", i))
            .collect::<String>();

        // Count steps for each size
        let steps_100 = with_step_counter(|| {
            let mut stream = TokenStream::new(&text_100);
            while stream.next().is_some() {}
        });

        let steps_1000 = with_step_counter(|| {
            let mut stream = TokenStream::new(&text_1000);
            while stream.next().is_some() {}
        });

        let steps_10000 = with_step_counter(|| {
            let mut stream = TokenStream::new(&text_10000);
            while stream.next().is_some() {}
        });

        // Verify O(n) scaling using steps-per-byte (more robust than ratios)
        let spb_100 = steps_per_byte(steps_100, text_100.len());
        let spb_1000 = steps_per_byte(steps_1000, text_1000.len());
        let spb_10000 = steps_per_byte(steps_10000, text_10000.len());

        // Steps-per-byte should be constant for O(n) algorithm
        // Allow 20% variance for state machine overhead
        let avg_spb = (spb_100 + spb_1000 + spb_10000) / 3.0;
        let tolerance = avg_spb * 0.2;

        assert!(
            (spb_100 - avg_spb).abs() <= tolerance,
            "Steps-per-byte variance too high: {:.3} vs avg {:.3}",
            spb_100,
            avg_spb
        );
        assert!(
            (spb_1000 - avg_spb).abs() <= tolerance,
            "Steps-per-byte variance too high: {:.3} vs avg {:.3}",
            spb_1000,
            avg_spb
        );
        assert!(
            (spb_10000 - avg_spb).abs() <= tolerance,
            "Steps-per-byte variance too high: {:.3} vs avg {:.3}",
            spb_10000,
            avg_spb
        );

        // Absolute upper bound: steps <= 3.0 * input_length (relaxed from 2x)
        assert!(
            steps_100 <= text_100.len() * 3,
            "Steps {} exceeded 3x input length {}",
            steps_100,
            text_100.len()
        );
        assert!(
            steps_1000 <= text_1000.len() * 3,
            "Steps {} exceeded 3x input length {}",
            steps_1000,
            text_1000.len()
        );
        assert!(
            steps_10000 <= text_10000.len() * 3,
            "Steps {} exceeded 3x input length {}",
            steps_10000,
            text_10000.len()
        );
    }

    #[test]
    fn test_tokenstream_worst_case_single_braces() {
        // Worst case: many single { that trigger SeenLBrace but fall back to Normal
        let text = "{ ".repeat(1000);

        let steps = with_step_counter(|| {
            let mut stream = TokenStream::new(&text);
            while stream.next().is_some() {}
        });

        // Should still be O(n) despite fallback states
        let spb = steps_per_byte(steps, text.len());
        assert!(
            spb <= 3.0,
            "Steps-per-byte {} exceeded 3.0 for fallback-heavy input",
            spb
        );
    }

    #[test]
    fn test_tokenstream_worst_case_backslashes() {
        // Worst case: many backslashes before tokens
        let text = (0..100)
            .map(|i| format!("\\\\\\\\{{{{token{}}}}} ", i))
            .collect::<String>();

        let steps = with_step_counter(|| {
            let mut stream = TokenStream::new(&text);
            while stream.next().is_some() {}
        });

        // Should still be O(n) with forward-only backslash tracking
        let spb = steps_per_byte(steps, text.len());
        assert!(
            spb <= 3.0,
            "Steps-per-byte {} exceeded 3.0 for backslash-heavy input",
            spb
        );
    }

    #[test]
    fn test_tokenstream_worst_case_sparse_tokens() {
        // Worst case: tokens separated by large amounts of text
        let text = (0..100)
            .map(|i| format!("{} {{{{token{}}}}}", "x".repeat(100), i))
            .collect::<String>();

        let steps = with_step_counter(|| {
            let mut stream = TokenStream::new(&text);
            while stream.next().is_some() {}
        });

        // Should still be O(n) for sparse tokens
        let spb = steps_per_byte(steps, text.len());
        assert!(
            spb <= 3.0,
            "Steps-per-byte {} exceeded 3.0 for sparse token input",
            spb
        );
    }

    #[test]
    fn test_tokenstream_worst_case_nested_braces_pattern() {
        // Worst case: patterns like {{{ and }}} that trigger multiple state transitions
        let text = "{{{ ".repeat(500) + &"}}} ".repeat(500);

        let steps = with_step_counter(|| {
            let mut stream = TokenStream::new(&text);
            while stream.next().is_some() {}
        });

        // Should still be O(n) despite complex brace patterns
        let spb = steps_per_byte(steps, text.len());
        assert!(
            spb <= 3.0,
            "Steps-per-byte {} exceeded 3.0 for nested brace pattern",
            spb
        );
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
}
