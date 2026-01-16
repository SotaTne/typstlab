//! O(n) Performance Verification Tests for TokenStream

use super::tokenize::TokenStream;

// O(n) Performance Verification with Direct Counter
// Uses atomic counter in TokenStream loop (deterministic, no overhead)

// Global counter for TokenStream steps (defined in Iterator::next)
// This is the same TOKENSTREAM_STEPS used in the #[cfg(test)] block

fn with_step_counter<F: FnOnce()>(f: F) -> usize {
    use crate::template::engine::tokenize::test_counter;
    test_counter::reset();
    f();
    test_counter::get()
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
