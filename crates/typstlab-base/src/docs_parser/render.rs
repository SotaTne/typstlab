use std::io::Read;

use super::DocsRenderError;
use super::body::body_to_markdown;
use super::route::route_to_relative_path;
use super::schema::DocsEntry;
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
        markdown.push_str(&body_to_markdown(body)?);
        markdown.push('\n');
    }

    Ok(markdown)
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
    fn test_parse_docs_json_tolerates_unknown_fields() {
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
        assert!(entries[0].extra.contains_key("future_field"));
    }

    #[test]
    fn test_render_docs_from_reader_into_reports_invalid_json() {
        let mut sink = MemorySink::default();

        let err = render_docs_from_reader_into(&b"not json"[..], &mut sink).unwrap_err();

        assert!(matches!(err, DocsRenderError::Json(_)));
        assert!(sink.files.is_empty());
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
