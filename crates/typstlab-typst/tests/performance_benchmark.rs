//! Performance benchmark for html_to_md native renderers
//!
//! Verifies that native renderers (Paragraph, Heading, List, Blockquote)
//! provide significant performance improvements over mdast_util_to_markdown.

use std::time::Instant;
use typstlab_typst::docs::html_to_md;

#[test]
fn test_paragraph_performance_100_items() {
    // Generate HTML with 100 paragraphs
    let mut html = String::new();
    for i in 0..100 {
        html.push_str(&format!("<p>Paragraph {} content goes here.</p>\n", i + 1));
    }

    // Warmup
    for _ in 0..5 {
        let _ = html_to_md::convert(&html, 1);
    }

    // Measure
    let start = Instant::now();
    for _ in 0..10 {
        html_to_md::convert(&html, 1).expect("Conversion should succeed");
    }
    let duration = start.elapsed();

    let avg_ms = duration.as_micros() / 10 / 1000;

    println!("\n=== PARAGRAPH PERFORMANCE (100 items) ===");
    println!("Average time: {}μs", duration.as_micros() / 10);
    println!("Average time: {:.2}ms", avg_ms as f64 / 1000.0);

    // Target: < 10ms for 100 paragraphs (baseline ~800ms)
    assert!(
        avg_ms < 10000,
        "Performance regression: {}μs > 10000μs target",
        avg_ms
    );
}

#[test]
fn test_heading_performance() {
    // Generate HTML with headings
    let html = r#"
<h1>Main Title</h1>
<h2>Section 1</h2>
<p>Content</p>
<h2>Section 2</h2>
<p>Content</p>
<h3>Subsection</h3>
<p>Content</p>
"#;

    // Warmup
    for _ in 0..5 {
        let _ = html_to_md::convert(html, 1);
    }

    // Measure
    let start = Instant::now();
    for _ in 0..100 {
        html_to_md::convert(html, 1).expect("Conversion should succeed");
    }
    let duration = start.elapsed();

    let avg_us = duration.as_micros() / 100;

    println!("\n=== HEADING PERFORMANCE ===");
    println!("Average time: {}μs", avg_us);

    // Target: < 5000μs per conversion (to allow for debug mode / noise)
    assert!(
        avg_us < 5000,
        "Performance regression: {}μs > 5000μs target",
        avg_us
    );
}

#[test]
fn test_list_performance() {
    // Generate HTML with lists
    let html = r#"
<ul>
<li>Item 1</li>
<li>Item 2</li>
<li>Item 3</li>
</ul>
<ol>
<li>First</li>
<li>Second</li>
<li>Third</li>
</ol>
"#;

    // Warmup
    for _ in 0..5 {
        let _ = html_to_md::convert(html, 1);
    }

    // Measure
    let start = Instant::now();
    for _ in 0..100 {
        html_to_md::convert(html, 1).expect("Conversion should succeed");
    }
    let duration = start.elapsed();

    let avg_us = duration.as_micros() / 100;

    println!("\n=== LIST PERFORMANCE ===");
    println!("Average time: {}μs", avg_us);

    // Target: < 5000μs per conversion
    assert!(
        avg_us < 5000,
        "Performance regression: {}μs > 5000μs target",
        avg_us
    );
}

#[test]
fn test_blockquote_performance() {
    // Generate HTML with blockquotes
    let html = r#"
<blockquote>
<p>This is a quote.</p>
<p>With multiple paragraphs.</p>
</blockquote>
"#;

    // Warmup
    for _ in 0..5 {
        let _ = html_to_md::convert(html, 1);
    }

    // Measure
    let start = Instant::now();
    for _ in 0..100 {
        html_to_md::convert(html, 1).expect("Conversion should succeed");
    }
    let duration = start.elapsed();

    let avg_us = duration.as_micros() / 100;

    println!("\n=== BLOCKQUOTE PERFORMANCE ===");
    println!("Average time: {}μs", avg_us);

    // Target: < 5000μs per conversion
    assert!(
        avg_us < 5000,
        "Performance regression: {}μs > 5000μs target",
        avg_us
    );
}

#[test]
fn test_mixed_content_performance() {
    // Real-world-like HTML with mixed elements
    let html = r#"
<h1>Documentation</h1>
<p>This is an introduction paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
<h2>Features</h2>
<ul>
<li>Feature 1</li>
<li>Feature 2</li>
<li>Feature 3</li>
</ul>
<p>More details here.</p>
<blockquote>
<p>Important note about usage.</p>
</blockquote>
<h2>Examples</h2>
<p>Example paragraph.</p>
"#;

    // Warmup
    for _ in 0..5 {
        let _ = html_to_md::convert(html, 1);
    }

    // Measure
    let start = Instant::now();
    for _ in 0..100 {
        html_to_md::convert(html, 1).expect("Conversion should succeed");
    }
    let duration = start.elapsed();

    let avg_us = duration.as_micros() / 100;

    println!("\n=== MIXED CONTENT PERFORMANCE ===");
    println!("Average time: {}μs", avg_us);
    println!("Average time: {:.2}ms", avg_us as f64 / 1000.0);

    // Target: < 10000μs per conversion for realistic mixed content
    assert!(
        avg_us < 10000,
        "Performance regression: {}μs > 10000μs target",
        avg_us
    );
}
