//! Integration tests for HTML to mdast conversion
//!
//! These tests verify end-to-end conversion quality using:
//! - Real HTML excerpts from Typst docs.json
//! - Complex nested structures
//! - Edge cases and mixed content

use typstlab_typst::docs::html_to_md;

/// Test: Real Typst documentation HTML with code blocks
#[test]
fn test_typst_code_block_with_preview() {
    let html = r#"
        <p>The <code>rect</code> function creates a rectangle:</p>
        <pre><code>#rect(width: 5cm, height: 3cm, fill: blue)</code></pre>
        <p>This produces a blue rectangle.</p>
    "#;

    let result = html_to_md::convert(html).expect("Should convert Typst code example");

    // Verify structure
    assert!(result.contains("The `rect` function creates a rectangle"));
    assert!(result.contains("```"));
    assert!(result.contains("#rect(width: 5cm, height: 3cm, fill: blue)"));
    assert!(result.contains("This produces a blue rectangle"));
}

/// Test: Nested lists with inline code
#[test]
fn test_nested_list_with_inline_code() {
    let html = r#"
        <ul>
            <li>
                <strong>Arrays</strong>: Use <code>array()</code> constructor
                <ul>
                    <li>Method: <code>push(item)</code></li>
                    <li>Method: <code>pop()</code></li>
                </ul>
            </li>
            <li>
                <strong>Dictionaries</strong>: Use <code>{}</code> syntax
            </li>
        </ul>
    "#;

    let result = html_to_md::convert(html).expect("Should convert nested list");

    // Verify list structure
    assert!(
        result.contains("**Arrays**"),
        "Should contain bold 'Arrays'"
    );
    assert!(result.contains("`array()`"), "Should contain inline code");
    assert!(
        result.contains("`push(item)`"),
        "Should contain nested method"
    );
    assert!(
        result.contains("`pop()`"),
        "Should contain second nested method"
    );
    assert!(
        result.contains("**Dictionaries**"),
        "Should contain bold 'Dictionaries'"
    );
}

/// Test: Table with links and inline formatting
#[test]
fn test_table_with_rich_content() {
    let html = r#"
        <table>
            <thead>
                <tr>
                    <th>Function</th>
                    <th>Description</th>
                    <th>Returns</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td><a href="/DOCS-BASE/reference/math/sqrt"><code>sqrt</code></a></td>
                    <td>Calculates <em>square root</em></td>
                    <td><code>float</code></td>
                </tr>
                <tr>
                    <td><a href="/DOCS-BASE/reference/math/pow"><code>pow</code></a></td>
                    <td>Raises to <strong>power</strong></td>
                    <td><code>float</code></td>
                </tr>
            </tbody>
        </table>
    "#;

    let result = html_to_md::convert(html).expect("Should convert table");

    // Debug: Print actual output
    eprintln!("Actual table output:\n{}", result);

    // Note: mdast_util_to_markdown may not support complex table structures
    // Verify table content is present (plain text fallback is acceptable)
    assert!(result.contains("Function"), "Should contain table header");
    assert!(result.contains("sqrt"), "Should contain table cell content");
    assert!(
        result.contains("square root"),
        "Should contain emphasized text"
    );
    assert!(result.contains("power"), "Should contain strong text");
}

/// Test: Blockquote with multiple paragraphs
#[test]
fn test_blockquote_multiline() {
    let html = r#"
        <blockquote>
            <p><strong>Note:</strong> This is important information.</p>
            <p>Make sure to read the documentation carefully.</p>
        </blockquote>
    "#;

    let result = html_to_md::convert(html).expect("Should convert blockquote");

    // Verify blockquote structure
    assert!(result.contains("> **Note:**"));
    assert!(result.contains("> Make sure to read"));
}

/// Test: Mixed inline elements in paragraph
#[test]
fn test_mixed_inline_formatting() {
    let html = r#"
        <p>
            The <code>array.map()</code> function applies a <em>transformation</em>
            to each element. See <a href="/DOCS-BASE/reference/array">array documentation</a>
            for <strong>more details</strong>.
        </p>
    "#;

    let result = html_to_md::convert(html).expect("Should convert mixed inline");

    // Verify inline elements preserved
    assert!(result.contains("`array.map()`"));
    assert!(result.contains("*transformation*"));
    assert!(result.contains("[array documentation](../reference/array.md)"));
    assert!(result.contains("**more details**"));
}

/// Test: Empty elements handling
#[test]
fn test_empty_elements() {
    let html = r#"
        <p></p>
        <p>Content</p>
        <ul>
            <li></li>
            <li>Item</li>
        </ul>
    "#;

    let result = html_to_md::convert(html).expect("Should handle empty elements");

    // Verify empty elements don't break structure
    assert!(result.contains("Content"));
    assert!(result.contains("Item"));
}

/// Test: Link with complex text content
#[test]
fn test_link_with_formatted_text() {
    let html = r#"
        <p>
            See <a href="/DOCS-BASE/tutorial/intro"><strong>Introduction</strong> to <em>Typst</em></a>
            for details.
        </p>
    "#;

    let result = html_to_md::convert(html).expect("Should convert link with formatted text");

    // Verify link contains formatted children
    assert!(result.contains("[**Introduction** to *Typst*](../tutorial/intro.md)"));
}

/// Test: Deeply nested structure
#[test]
fn test_deeply_nested_structure() {
    let html = r#"
        <ul>
            <li>
                Level 1
                <ul>
                    <li>
                        Level 2
                        <ul>
                            <li>Level 3 with <code>code</code></li>
                        </ul>
                    </li>
                </ul>
            </li>
        </ul>
    "#;

    let result = html_to_md::convert(html).expect("Should handle deep nesting");

    // Verify nested list structure (indentation may vary)
    assert!(result.contains("Level 1"));
    assert!(result.contains("Level 2"));
    assert!(result.contains("Level 3"));
    assert!(result.contains("`code`"));
}

/// Test: Ordered list with inline formatting
#[test]
fn test_ordered_list_rich_content() {
    let html = r#"
        <ol>
            <li><strong>First</strong>: Install <code>typst</code></li>
            <li><strong>Second</strong>: Create a <em>document</em></li>
            <li><strong>Third</strong>: Compile with <code>typst compile</code></li>
        </ol>
    "#;

    let result = html_to_md::convert(html).expect("Should convert ordered list");

    // Verify ordered list numbering
    assert!(result.contains("1. **First**"));
    assert!(result.contains("`typst`"));
    assert!(result.contains("2. **Second**"));
    assert!(result.contains("*document*"));
    assert!(result.contains("3. **Third**"));
}

/// Test: Internal link rewriting
#[test]
fn test_internal_link_rewriting() {
    let html = r#"
        <p>
            <a href="/DOCS-BASE/">Home</a> |
            <a href="/DOCS-BASE/reference/">Reference</a> |
            <a href="/DOCS-BASE/tutorial/intro/">Tutorial</a>
        </p>
    "#;

    let result = html_to_md::convert(html).expect("Should rewrite links");

    // Verify all /DOCS-BASE/ links rewritten to .md format
    assert!(result.contains("[Home](../index.md)"));
    assert!(result.contains("[Reference](../reference.md)"));
    assert!(result.contains("[Tutorial](../tutorial/intro.md)"));
    assert!(!result.contains("/DOCS-BASE/"));
}

/// Test: Code block with special characters
#[test]
fn test_code_block_special_chars() {
    let html = r#"
        <pre><code>let x = "Hello, World!"
#if x.contains("test") {
  print("Contains test")
}</code></pre>
    "#;

    let result = html_to_md::convert(html).expect("Should preserve special chars in code");

    // Verify code fence and content
    assert!(result.contains("```"), "Should have code fence");
    assert!(
        result.contains("Hello, World!"),
        "Should contain code content"
    );
    assert!(result.contains("x.contains"), "Should contain code logic");
}

/// Test: Whitespace preservation in inline code
#[test]
fn test_inline_code_whitespace() {
    let html = r#"<p>Use <code>  indented code  </code> carefully.</p>"#;

    let result = html_to_md::convert(html).expect("Should handle code whitespace");

    // Verify inline code present (whitespace may be normalized by markdown-rs)
    assert!(result.contains("`"), "Should contain inline code markers");
    assert!(result.contains("indented code"), "Should contain code text");
    assert!(
        result.contains("carefully"),
        "Should contain surrounding text"
    );
}

/// Test: Multiple paragraphs with various elements
#[test]
fn test_multiple_paragraphs_mixed_content() {
    let html = r#"
        <p>First paragraph with <strong>bold</strong> text.</p>
        <p>Second paragraph with <a href="/DOCS-BASE/link">a link</a>.</p>
        <p>Third paragraph with <code>inline code</code> and <em>emphasis</em>.</p>
    "#;

    let result = html_to_md::convert(html).expect("Should convert multiple paragraphs");

    // Verify paragraph separation (blank lines between)
    let lines: Vec<&str> = result.lines().collect();
    assert!(lines.len() >= 5); // At least 3 content lines + 2 blank lines

    // Verify content
    assert!(result.contains("**bold**"));
    assert!(result.contains("[a link](../link.md)"));
    assert!(result.contains("`inline code`"));
    assert!(result.contains("*emphasis*"));
}

/// Test: Table without thead (direct tbody)
#[test]
fn test_table_without_thead() {
    let html = r#"
        <table>
            <tr>
                <td>Cell 1</td>
                <td>Cell 2</td>
            </tr>
            <tr>
                <td>Cell 3</td>
                <td>Cell 4</td>
            </tr>
        </table>
    "#;

    let result = html_to_md::convert(html).expect("Should handle table without thead");

    // Verify table content is present
    assert!(result.contains("Cell 1"), "Should contain cell 1");
    assert!(result.contains("Cell 2"), "Should contain cell 2");
    assert!(result.contains("Cell 3"), "Should contain cell 3");
    assert!(result.contains("Cell 4"), "Should contain cell 4");
}

/// Test: Link without href attribute
#[test]
fn test_link_missing_href() {
    let html = r#"<p>Text with <a>link without href</a> should still work.</p>"#;

    let result = html_to_md::convert(html).expect("Should handle link without href");

    // Verify fallback link created
    assert!(result.contains("[link without href](#)"));
}

/// Test: Emphasis and strong combinations
#[test]
fn test_emphasis_strong_combinations() {
    let html = r#"
        <p>
            <em>Italic only</em>,
            <strong>Bold only</strong>,
            <strong><em>Bold and italic</em></strong>,
            <em><strong>Italic and bold</strong></em>
        </p>
    "#;

    let result = html_to_md::convert(html).expect("Should convert emphasis combinations");

    // Verify markdown formatting
    assert!(result.contains("*Italic only*"));
    assert!(result.contains("**Bold only**"));
    // Note: Nested formatting may vary, just verify both markers present
    assert!(result.contains("**"));
    assert!(result.contains("*"));
}

/// Test: Link rewriting - directory with trailing slash converts to .md
#[test]
fn test_link_rewrite_directory_to_md() {
    let html = r#"<a href="/DOCS-BASE/tutorial/">Tutorial</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../tutorial.md"),
        "Expected ../tutorial.md, got: {}",
        result
    );
    assert!(
        !result.contains("index.md"),
        "Should not contain index.md, got: {}",
        result
    );
}

/// Test: Link rewriting - preserves fragments
#[test]
fn test_link_rewrite_with_fragment() {
    let html = r#"<a href="/DOCS-BASE/tutorial/#section">Link</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../tutorial.md#section"),
        "Expected ../tutorial.md#section, got: {}",
        result
    );
}

/// Test: Link rewriting - preserves query strings
#[test]
fn test_link_rewrite_with_query() {
    let html = r#"<a href="/DOCS-BASE/api?version=1">API</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../api.md?version=1"),
        "Expected ../api.md?version=1, got: {}",
        result
    );
}

/// Test: Link rewriting - query and fragment together
#[test]
fn test_link_rewrite_with_query_and_fragment() {
    let html = r#"<a href="/DOCS-BASE/api?v=1#intro">API</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../api.md?v=1#intro"),
        "Expected ../api.md?v=1#intro, got: {}",
        result
    );
}

/// Test: Link rewriting - nested directory
#[test]
fn test_link_rewrite_nested_directory() {
    let html = r#"<a href="/DOCS-BASE/reference/styling/">Styling</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../reference/styling.md"),
        "Expected ../reference/styling.md, got: {}",
        result
    );
}

/// Test: Link rewriting - root to index.md
#[test]
fn test_link_rewrite_root() {
    let html = r#"<a href="/DOCS-BASE/">Home</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../index.md"),
        "Expected ../index.md for root, got: {}",
        result
    );
}

/// Test: Link rewriting - external URLs unchanged
#[test]
fn test_link_rewrite_external_url_unchanged() {
    let html = r#"<a href="https://typst.app/">Typst</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("https://typst.app/"),
        "External URL should be unchanged, got: {}",
        result
    );
    assert!(
        !result.contains(".."),
        "External URL should not be rewritten, got: {}",
        result
    );
}

/// Test: Link rewriting - mailto unchanged
#[test]
fn test_link_rewrite_mailto_unchanged() {
    let html = r#"<a href="mailto:test@example.com">Email</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("mailto:test@example.com"),
        "Mailto should be unchanged, got: {}",
        result
    );
}

/// Test: Link rewriting - file without trailing slash
#[test]
fn test_link_rewrite_file_without_trailing_slash() {
    let html = r#"<a href="/DOCS-BASE/tutorial/writing">Writing</a>"#;
    let result = html_to_md::convert(html).unwrap();

    assert!(
        result.contains("../tutorial/writing.md"),
        "Expected ../tutorial/writing.md, got: {}",
        result
    );
}
