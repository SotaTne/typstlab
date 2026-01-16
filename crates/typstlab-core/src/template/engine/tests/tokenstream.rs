//! Unit tests for TokenStream

use super::tokenize::{ScanState, TokenKind, TokenStream};

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
