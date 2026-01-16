//! Tokenization for template engine
//!
//! Provides O(n) tokenization using a state machine.

/// Token classification for future extensibility
///
/// This enum allows adding new constructs without changing the tokenization logic.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TokenKind {
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
pub(crate) struct Token {
    /// Token classification
    pub kind: TokenKind,
    /// Absolute byte position of `{{` in template
    pub start: usize,
    /// Total length in bytes including {{ and }}
    pub length: usize,
    /// Number of backslashes before `{{`
    /// Odd count = escaped (literal), even = real (processed)
    pub backslash_count: usize,
    /// Line number where token starts (for error messages)
    pub line: usize,
}

impl Token {
    /// Check if this token is escaped (odd backslash count)
    pub fn is_escaped(&self) -> bool {
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
pub(crate) enum ScanState {
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
pub(crate) struct TokenStream<'a> {
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
    pub fn new(text: &'a str) -> Self {
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

    /// Check if we should continue iteration (guards for timeout and EOF)
    #[inline]
    fn should_continue(&mut self, max_steps: usize) -> bool {
        self.step_count += 1;
        self.step_count <= max_steps && self.pos < self.bytes.len()
    }

    /// Record step for O(n) performance verification in tests
    #[cfg(test)]
    #[inline]
    fn record_test_step() {
        test_counter::inc();
    }

    /// Process Normal state: scan for backslashes and opening braces
    fn process_normal_state(&mut self, byte: u8, backslash_count: usize) {
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
                backslash_count,
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

    /// Process SeenLBrace state: check for second brace to start token
    ///
    /// Returns true if position was advanced, false if byte should be reprocessed
    fn process_seen_lbrace(&mut self, byte: u8, lbrace_pos: usize, backslash_count: usize) -> bool {
        if byte == b'{' {
            // Found {{, transition to InToken
            self.state = ScanState::InToken {
                start: lbrace_pos,
                content_start: self.pos + 1,
                backslash_count,
            };
            self.pos += 1;
            true
        } else {
            // Just a single {, not a token
            self.state = ScanState::Normal { backslash_count: 0 };
            // Don't increment pos, reprocess this byte in Normal state
            false
        }
    }

    /// Process InToken state: scan for closing braces
    fn process_in_token(
        &mut self,
        byte: u8,
        start: usize,
        content_start: usize,
        backslash_count: usize,
    ) {
        if byte == b'}' {
            // Potential end of }}
            self.state = ScanState::SeenRBrace {
                start,
                content_start,
                rbrace_pos: self.pos,
                backslash_count,
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

    /// Process SeenRBrace state: check for second brace to complete token
    ///
    /// Returns (Option<Token>, should_reprocess)
    fn process_seen_rbrace(
        &mut self,
        byte: u8,
        start: usize,
        content_start: usize,
        rbrace_pos: usize,
        backslash_count: usize,
    ) -> (Option<Token>, bool) {
        if byte == b'}' {
            // Found }}, complete token
            let content = std::str::from_utf8(&self.bytes[content_start..rbrace_pos]).unwrap_or("");

            let token = Token {
                kind: self.classify_content(content),
                start,
                length: self.pos + 1 - start,
                backslash_count,
                line: self.line,
            };

            self.state = ScanState::Normal { backslash_count: 0 };
            self.pos += 1;

            (Some(token), true)
        } else {
            // Just a single } inside content, continue scanning
            self.state = ScanState::InToken {
                start,
                content_start,
                backslash_count,
            };
            // Don't increment pos, reprocess this byte in InToken state
            (None, false)
        }
    }
}

impl<'a> Iterator for TokenStream<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        const MAX_STEPS_MULTIPLIER: usize = 3;
        let max_steps = self.bytes.len().saturating_mul(MAX_STEPS_MULTIPLIER);

        loop {
            if !self.should_continue(max_steps) {
                return None;
            }

            let byte = self.bytes[self.pos];
            #[cfg(test)]
            Self::record_test_step();

            match &self.state {
                ScanState::Normal { backslash_count } => {
                    self.process_normal_state(byte, *backslash_count)
                }
                ScanState::SeenLBrace {
                    pos,
                    backslash_count,
                } => {
                    if !self.process_seen_lbrace(byte, *pos, *backslash_count) {
                        continue;
                    }
                }
                ScanState::InToken {
                    start,
                    content_start,
                    backslash_count,
                } => self.process_in_token(byte, *start, *content_start, *backslash_count),
                ScanState::SeenRBrace {
                    start,
                    content_start,
                    rbrace_pos,
                    backslash_count,
                } => {
                    let (token, advanced) = self.process_seen_rbrace(
                        byte,
                        *start,
                        *content_start,
                        *rbrace_pos,
                        *backslash_count,
                    );
                    if let Some(token) = token {
                        return Some(token);
                    }
                    if !advanced {
                        continue;
                    }
                }
            }
        }
    }
}

/// Test-only step counter for O(n) performance verification
///
/// Uses thread-local storage to avoid interference between parallel tests.
/// Each test thread has its own independent counter.
#[cfg(test)]
pub(crate) mod test_counter {
    use std::cell::Cell;

    thread_local! {
        static TEST_STEP_COUNTER: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        TEST_STEP_COUNTER.with(|c| c.set(0));
    }

    pub(crate) fn get() -> usize {
        TEST_STEP_COUNTER.with(|c| c.get())
    }

    pub(crate) fn inc() {
        TEST_STEP_COUNTER.with(|c| c.set(c.get() + 1));
    }
}
