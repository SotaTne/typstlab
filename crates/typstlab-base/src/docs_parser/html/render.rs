use super::ast::Html;
use super::error::HtmlRenderError;
use crate::docs_parser::html::markdown::{MarkdownContext, ToMarkdownDocument};
use crate::docs_parser::md::ToMarkdown;

pub fn html_to_markdown(html: &Html) -> Result<String, HtmlRenderError> {
    Ok(trim_markdown(
        html.root.to_markdown_document()?.to_markdown(),
    ))
}

pub fn html_to_markdown_with_source_route(
    html: &Html,
    source_route: &str,
) -> Result<String, HtmlRenderError> {
    let context = MarkdownContext {
        source_route: Some(source_route.to_string()),
    };
    Ok(trim_markdown(
        html.root
            .to_markdown_document_with_context(&context)?
            .to_markdown(),
    ))
}

fn trim_markdown(markdown: String) -> String {
    markdown.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_html_to_markdown() {
        let html =
            Html::parse(r#"<p>Hello <strong>world</strong> <a href="/x">link</a>.</p>"#).unwrap();
        let markdown = html_to_markdown(&html).unwrap();

        assert!(markdown.contains("Hello"));
        assert!(markdown.contains("**world**"));
        assert!(markdown.contains("[link](/x)"));
    }

    #[test]
    fn renders_docs_base_links_relative_to_source_route() {
        let html = Html::parse(
            r#"<p>Fixed crash when <a href="/DOCS-BASE/reference/foundations/symbol/" title="`symbol`">symbol</a> function was called.</p>"#,
        )
        .unwrap();
        let markdown =
            html_to_markdown_with_source_route(&html, "/DOCS-BASE/changelog/0.1/").unwrap();

        assert!(markdown.contains("[symbol](../reference/foundations/symbol.md \"`symbol`\")"));
        assert!(!markdown.contains("/DOCS-BASE/"));
    }

    #[test]
    fn renders_code_block() {
        let html = Html::parse("<pre><code>let x = 1;</code></pre>").unwrap();
        let markdown = html_to_markdown(&html).unwrap();

        assert!(markdown.contains("```"));
        assert!(markdown.contains("let x = 1;"));
    }

    #[test]
    fn renders_multiline_code_tag_as_code_block() {
        let html = Html::parse(
            r#"<code>// Don't do this
#text(
  size: 16pt,
  weight: "bold",
)[Heading]
</code>"#,
        )
        .unwrap();
        let markdown = html_to_markdown(&html).unwrap();

        assert!(markdown.starts_with("```"));
        assert!(markdown.contains("#text("));
        assert!(markdown.ends_with("```"));
        assert!(!markdown.starts_with("`//"));
    }

    #[test]
    fn renders_changelog_list_items_without_empty_headings() {
        let html = Html::parse(
            r#"<h1>Version 0.1.0 (April 04, 2023)</h1>
<h2 id="breaking-changes">Breaking changes</h2>
<ul>
<li>When using the CLI, you now have to use subcommands:
<ul>
<li><code>typst compile file.typ</code> or <code>typst c file.typ</code> to create a PDF</li>
<li><code>typst watch file.typ</code> or <code>typst w file.typ</code> to compile and watch</li>
<li><code>typst fonts</code> to list all fonts</li>
</ul>
</li>
</ul>"#,
        )
        .unwrap();
        let markdown = html_to_markdown(&html).unwrap();

        assert!(markdown.contains("# Version 0.1.0"));
        assert!(markdown.contains("## Breaking changes"));
        assert!(!markdown.contains("# \n"));
        assert!(!markdown.contains("## \n"));
        assert!(markdown.contains("- When using the CLI"));
        assert!(markdown.contains("  - `typst compile file.typ`"));
    }
}
