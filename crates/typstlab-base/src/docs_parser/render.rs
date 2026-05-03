use std::io::Read;

use super::DocsRenderError;
use super::html::{Html, HtmlRenderError};
use super::route::{route_to_relative_link, route_to_relative_path};
use super::schema::{
    CategoryContent, CategoryItem, DocsBody, DocsEntry, FuncContent, GroupContent, ParamContent,
    RichBlock, RichContent, SymbolItem, SymbolsContent, TypeContent,
};
use super::sink::{DocsRenderSink, RenderedDocs, TempDocsRenderSink};

pub fn parse_docs_json_from_reader<R>(reader: R) -> Result<Vec<DocsEntry>, serde_json::Error>
where
    R: Read,
{
    serde_json::from_reader(reader)
}

pub fn render_docs_from_reader_into<R, S>(reader: R, sink: &mut S) -> Result<usize, DocsRenderError>
where
    R: Read,
    S: DocsRenderSink,
{
    let entries = parse_docs_json_from_reader(reader)?;
    render_docs_into(&entries, sink)
}

pub fn render_docs_into<S>(entries: &[DocsEntry], sink: &mut S) -> Result<usize, DocsRenderError>
where
    S: DocsRenderSink,
{
    let mut count = 0;
    for entry in entries {
        count += render_entry_into(entry, sink)?;
    }
    Ok(count)
}

pub fn render_docs_from_reader<R>(reader: R) -> Result<RenderedDocs, DocsRenderError>
where
    R: Read,
{
    let mut sink = TempDocsRenderSink::new()?;
    render_docs_from_reader_into(reader, &mut sink)?;
    Ok(sink.into_rendered_docs())
}

fn render_entry_into<S>(entry: &DocsEntry, sink: &mut S) -> Result<usize, DocsRenderError>
where
    S: DocsRenderSink,
{
    let relative_path = route_to_relative_path(&entry.route)?;
    let markdown = entry_to_markdown(entry)?;
    sink.write_markdown(&relative_path, &markdown)?;

    let mut count = 1;
    for child in &entry.children {
        count += render_entry_into(child, sink)?;
    }
    Ok(count)
}

fn entry_to_markdown(entry: &DocsEntry) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    markdown.push_str("---\n");
    markdown.push_str("title: ");
    markdown.push_str(&entry.title);
    markdown.push('\n');
    if let Some(description) = &entry.description {
        markdown.push_str("description: ");
        markdown.push_str(description);
        markdown.push('\n');
    }
    markdown.push_str("---\n\n");

    if let Some(body) = &entry.body {
        let body_markdown = body_to_markdown(entry, body)?;
        if body_markdown.trim().is_empty() {
            return Err(DocsRenderError::Body(format!(
                "body rendered empty for route {}",
                entry.route
            )));
        }
        markdown.push_str(&body_markdown);
        markdown.push('\n');
    }

    Ok(markdown)
}

fn body_to_markdown(entry: &DocsEntry, body: &DocsBody) -> Result<String, DocsRenderError> {
    match body {
        DocsBody::Html(content) => Ok(content.to_markdown_with_source_route(&entry.route)?),
        DocsBody::Func(content) => func_to_markdown(content, &entry.route),
        DocsBody::Type(content) => type_to_markdown(content, &entry.route),
        DocsBody::Category(content) => category_to_markdown(entry, content),
        DocsBody::Group(content) => group_to_markdown(content, &entry.route),
        DocsBody::Symbols(content) => symbols_to_markdown(content, &entry.route),
    }
}

fn category_to_markdown(
    entry: &DocsEntry,
    content: &CategoryContent,
) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    push_rich_content(&mut markdown, content.details.as_ref(), &entry.route)?;
    push_category_items(&mut markdown, &entry.route, "Items", &content.items)?;
    if let Some(shorthands) = &content.shorthands {
        push_symbol_items(&mut markdown, "Markup Shorthands", &shorthands.markup);
        push_symbol_items(&mut markdown, "Math Shorthands", &shorthands.math);
    }
    Ok(trim_section(markdown))
}

fn type_to_markdown(content: &TypeContent, source_route: &str) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    push_oneliner(&mut markdown, content.oneliner.as_deref());
    push_rich_content(&mut markdown, content.details.as_ref(), source_route)?;
    if let Some(constructor) = &content.constructor {
        push_func_summary(&mut markdown, "Constructor", constructor, source_route);
    }
    push_func_list(&mut markdown, "Definitions", &content.scope, source_route);
    Ok(trim_section(markdown))
}

fn func_to_markdown(content: &FuncContent, source_route: &str) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    push_oneliner(&mut markdown, content.oneliner.as_deref());
    push_rich_content(&mut markdown, content.details.as_ref(), source_route)?;
    push_params(&mut markdown, &content.params, source_route)?;
    push_returns(&mut markdown, &content.returns);
    push_func_list(&mut markdown, "Definitions", &content.scope, source_route);
    Ok(trim_section(markdown))
}

fn group_to_markdown(
    content: &GroupContent,
    source_route: &str,
) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    push_rich_content(&mut markdown, content.details.as_ref(), source_route)?;
    push_func_list(&mut markdown, "Functions", &content.functions, source_route);
    Ok(trim_section(markdown))
}

fn symbols_to_markdown(
    content: &SymbolsContent,
    source_route: &str,
) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    push_rich_content(&mut markdown, content.details.as_ref(), source_route)?;
    push_symbol_items(&mut markdown, "Symbols", &content.list);
    Ok(trim_section(markdown))
}

fn push_oneliner(markdown: &mut String, oneliner: Option<&str>) {
    if let Some(oneliner) = oneliner.filter(|value| !value.trim().is_empty()) {
        push_block(markdown, oneliner);
    }
}

fn push_rich_content(
    markdown: &mut String,
    content: Option<&RichContent>,
    source_route: &str,
) -> Result<(), DocsRenderError> {
    let Some(content) = content else {
        return Ok(());
    };

    match content {
        RichContent::Plain(value) => {
            push_block(markdown, &html_string_to_markdown(value, source_route)?);
        }
        RichContent::Blocks(blocks) => {
            for block in blocks {
                match block {
                    RichBlock::Html(html) => {
                        push_block(markdown, &html.to_markdown_with_source_route(source_route)?)
                    }
                    RichBlock::Example(example) => {
                        if let Some(title) = &example.title {
                            push_block(markdown, &format!("### {title}"));
                        }
                        push_block(
                            markdown,
                            &example.body.to_markdown_with_source_route(source_route)?,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn html_string_to_markdown(input: &str, source_route: &str) -> Result<String, DocsRenderError> {
    Html::parse(input)
        .map_err(|error| DocsRenderError::Html(HtmlRenderError::Parse(error)))?
        .to_markdown_with_source_route(source_route)
        .map_err(DocsRenderError::from)
}

fn push_category_items(
    markdown: &mut String,
    source_route: &str,
    title: &str,
    items: &[CategoryItem],
) -> Result<(), DocsRenderError> {
    if items.is_empty() {
        return Ok(());
    }

    push_block(markdown, &format!("## {title}"));
    for item in items {
        let name = if item.code {
            format!("`{}`", item.name)
        } else {
            item.name.clone()
        };
        let target = route_to_relative_link(source_route, &item.route)?;
        let mut line = format!("- [{name}]({})", target.display());
        if let Some(oneliner) = &item.oneliner {
            line.push_str(": ");
            line.push_str(oneliner);
        }
        markdown.push_str(&line);
        markdown.push('\n');
    }
    markdown.push('\n');
    Ok(())
}

fn push_func_summary(markdown: &mut String, title: &str, func: &FuncContent, source_route: &str) {
    push_block(markdown, &format!("## {title}"));
    push_func_item(markdown, func, source_route);
    markdown.push('\n');
}

fn push_func_list(
    markdown: &mut String,
    title: &str,
    functions: &[FuncContent],
    source_route: &str,
) {
    if functions.is_empty() {
        return;
    }

    push_block(markdown, &format!("## {title}"));
    for func in functions {
        push_func_item(markdown, func, source_route);
    }
    markdown.push('\n');
}

fn push_func_item(markdown: &mut String, func: &FuncContent, _source_route: &str) {
    let title = if func.title.is_empty() {
        &func.name
    } else {
        &func.title
    };
    let mut line = format!("- `{}`", func.name);
    if title != &func.name {
        line.push_str(" - ");
        line.push_str(title);
    }
    if let Some(oneliner) = &func.oneliner {
        line.push_str(": ");
        line.push_str(oneliner);
    }
    markdown.push_str(&line);
    markdown.push('\n');
}

fn push_params(
    markdown: &mut String,
    params: &[ParamContent],
    source_route: &str,
) -> Result<(), DocsRenderError> {
    if params.is_empty() {
        return Ok(());
    }

    push_block(markdown, "## Parameters");
    for param in params {
        let mut line = format!("- `{}`", param.name);
        if !param.types.is_empty() {
            line.push_str(" (");
            line.push_str(&param.types.join(", "));
            line.push(')');
        }
        if param.required {
            line.push_str(" required");
        }
        markdown.push_str(&line);
        markdown.push('\n');

        let details = rich_content_to_markdown(param.details.as_ref(), source_route)?;
        if !details.trim().is_empty() {
            for line in details.lines() {
                markdown.push_str("  ");
                markdown.push_str(line);
                markdown.push('\n');
            }
        }
    }
    markdown.push('\n');
    Ok(())
}

fn rich_content_to_markdown(
    content: Option<&RichContent>,
    source_route: &str,
) -> Result<String, DocsRenderError> {
    let mut markdown = String::new();
    push_rich_content(&mut markdown, content, source_route)?;
    Ok(trim_section(markdown))
}

fn push_returns(markdown: &mut String, returns: &[String]) {
    if returns.is_empty() {
        return;
    }

    push_block(markdown, "## Returns");
    markdown.push_str(&returns.join(", "));
    markdown.push_str("\n\n");
}

fn push_symbol_items(markdown: &mut String, title: &str, items: &[SymbolItem]) {
    if items.is_empty() {
        return;
    }

    push_block(markdown, &format!("## {title}"));
    markdown.push_str("| Name | Value | Markup | Math |\n");
    markdown.push_str("| --- | --- | --- | --- |\n");
    for item in items {
        markdown.push_str("| `");
        markdown.push_str(&escape_table_cell(&item.name));
        markdown.push_str("` | ");
        markdown.push_str(item.value.as_deref().unwrap_or(""));
        markdown.push_str(" | ");
        markdown.push_str(item.markup_shorthand.as_deref().unwrap_or(""));
        markdown.push_str(" | ");
        markdown.push_str(item.math_shorthand.as_deref().unwrap_or(""));
        markdown.push_str(" |\n");
    }
    markdown.push('\n');
}

fn push_block(markdown: &mut String, block: &str) {
    let block = block.trim();
    if block.is_empty() {
        return;
    }
    markdown.push_str(block);
    markdown.push_str("\n\n");
}

fn trim_section(markdown: String) -> String {
    markdown.trim().to_string()
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[derive(Debug, Default)]
    struct MemorySink {
        files: Vec<(PathBuf, String)>,
    }

    impl DocsRenderSink for MemorySink {
        fn write_markdown(
            &mut self,
            relative_path: &Path,
            content: &str,
        ) -> Result<(), DocsRenderError> {
            self.files
                .push((relative_path.to_path_buf(), content.to_string()));
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingSink;

    impl DocsRenderSink for FailingSink {
        fn write_markdown(
            &mut self,
            _relative_path: &Path,
            _content: &str,
        ) -> Result<(), DocsRenderError> {
            Err(DocsRenderError::Sink("memory sink failed".to_string()))
        }
    }

    #[test]
    fn test_render_docs_from_reader_into_writes_nested_docs_to_sink() {
        let json = br#"[
            {
                "route": "/DOCS-BASE/",
                "title": "Overview",
                "body": { "kind": "html", "content": "<p>Hello docs</p>" },
                "children": [
                    {
                        "route": "/DOCS-BASE/tutorial/writing/",
                        "title": "Writing",
                        "body": { "kind": "html", "content": "<p>Write text</p>" },
                        "children": []
                    }
                ]
            }
        ]"#;
        let mut sink = MemorySink::default();

        let count = render_docs_from_reader_into(&json[..], &mut sink).unwrap();

        assert_eq!(count, 2);
        assert_eq!(sink.files[0].0, Path::new("index.md"));
        assert!(sink.files[0].1.contains("title: Overview"));
        assert!(sink.files[0].1.contains("Hello docs"));
        assert_eq!(
            sink.files[1].0,
            PathBuf::from("tutorial").join("writing.md")
        );
        assert!(sink.files[1].1.contains("Write text"));
    }

    #[test]
    fn test_parse_docs_json_ignores_unknown_fields() {
        let json = br#"[
            {
                "route": "/DOCS-BASE/",
                "title": "Overview",
                "future_field": { "kept": true },
                "children": []
            }
        ]"#;

        let entries = parse_docs_json_from_reader(&json[..]).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Overview");
    }

    #[test]
    fn test_render_docs_from_reader_into_reports_invalid_json() {
        let mut sink = MemorySink::default();

        let err = render_docs_from_reader_into(&b"not json"[..], &mut sink).unwrap_err();

        assert!(matches!(err, DocsRenderError::Json(_)));
        assert!(sink.files.is_empty());
    }

    #[test]
    fn test_render_docs_from_reader_into_rejects_frontmatter_only_body() {
        let json = br#"[
            {
                "route": "/DOCS-BASE/reference/text/",
                "title": "Text",
                "body": {
                    "kind": "category",
                    "content": {
                        "name": "text",
                        "title": "Text",
                        "items": []
                    }
                },
                "children": []
            }
        ]"#;
        let mut sink = MemorySink::default();

        let err = render_docs_from_reader_into(&json[..], &mut sink).unwrap_err();

        assert!(
            matches!(err, DocsRenderError::Body(message) if message.contains("body rendered empty"))
        );
        assert!(sink.files.is_empty());
    }

    #[test]
    fn test_render_docs_from_reader_into_renders_category_body() {
        let json = br#"[
            {
                "route": "/DOCS-BASE/reference/text/",
                "title": "Text",
                "body": {
                    "kind": "category",
                    "content": {
                        "name": "text",
                        "title": "Text",
                        "details": "<p>Text styling.</p>",
                        "items": [
                            {
                                "name": "highlight",
                                "route": "/DOCS-BASE/reference/text/highlight/",
                                "oneliner": "Highlights text.",
                                "code": true
                            }
                        ]
                    }
                },
                "children": []
            }
        ]"#;
        let mut sink = MemorySink::default();

        render_docs_from_reader_into(&json[..], &mut sink).unwrap();

        let markdown = &sink.files[0].1;
        assert!(markdown.contains("Text styling."));
        assert!(markdown.contains("## Items"));
        assert!(markdown.contains("- [`highlight`](text/highlight.md): Highlights text."));
    }

    #[test]
    fn test_render_docs_from_reader_into_rejects_traversal_route() {
        let json = br#"[
            {
                "route": "/DOCS-BASE/../escape/",
                "title": "Escape",
                "children": []
            }
        ]"#;
        let mut sink = MemorySink::default();

        let err = render_docs_from_reader_into(&json[..], &mut sink).unwrap_err();

        assert!(matches!(err, DocsRenderError::PathTraversal(_)));
        assert!(sink.files.is_empty());
    }

    #[test]
    fn test_render_docs_from_reader_into_rejects_rooted_route() {
        let json = br#"[
            {
                "route": "/DOCS-BASE//tmp/evil/",
                "title": "Evil",
                "children": []
            }
        ]"#;
        let mut sink = MemorySink::default();

        let err = render_docs_from_reader_into(&json[..], &mut sink).unwrap_err();

        assert!(matches!(err, DocsRenderError::RootedPath(_)));
        assert!(sink.files.is_empty());
    }

    #[test]
    fn test_render_docs_from_reader_into_preserves_sink_error() {
        let json = br#"[
            {
                "route": "/DOCS-BASE/",
                "title": "Overview",
                "children": []
            }
        ]"#;
        let mut sink = FailingSink;

        let err = render_docs_from_reader_into(&json[..], &mut sink).unwrap_err();

        assert!(matches!(err, DocsRenderError::Sink(_)));
    }

    #[test]
    fn test_render_docs_from_reader_returns_tempdir_backed_docs() {
        let rendered_path;
        {
            let json = br#"[
                {
                    "route": "/DOCS-BASE/",
                    "title": "Overview",
                    "body": { "kind": "html", "content": "<p>Hello</p>" },
                    "children": []
                }
            ]"#;

            let rendered = render_docs_from_reader(&json[..]).unwrap();
            rendered_path = rendered.path().to_path_buf();

            let output = rendered.path().join("index.md");
            assert!(output.exists());
            assert!(std::fs::read_to_string(output).unwrap().contains("Hello"));
            assert_eq!(rendered.file_count(), 1);
        }

        assert!(
            !rendered_path.exists(),
            "RenderedDocs must clean up its TempDir on drop"
        );
    }
}
