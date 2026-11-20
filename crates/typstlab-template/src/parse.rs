use super::ast::{Node, NodeKind, ParamRef, Span, TemplateAst};
use super::error::{ParseErrorKind, TemplateEngineError};

const START_TAG: &str = "{{";
const END_TAG: &str = "}}";
const LITERAL_MARKER: char = '=';

pub fn parse_template(source: &str) -> Result<TemplateAst, TemplateEngineError> {
    let mut nodes = Vec::new();
    let mut cursor = 0;

    while let Some(relative_start) = source[cursor..].find(START_TAG) {
        let start = cursor + relative_start;
        let raw_start = cursor;
        let raw_end = start;
        if raw_start < raw_end {
            nodes.push(Node::new(
                NodeKind::Raw(source[raw_start..raw_end].to_string()),
                Span::new(raw_start, raw_end),
            ));
        }

        let tag_content_start = start + START_TAG.len();
        let after_open = &source[tag_content_start..];
        let Some(end) = after_open.find(END_TAG) else {
            return Err(parse_error(
                ParseErrorKind::UnclosedTag,
                Span::new(start, source.len()),
            ));
        };

        let tag_end = tag_content_start + end;
        let full_end = tag_end + END_TAG.len();
        let raw_tag = &source[tag_content_start..tag_end];
        nodes.push(parse_tag(raw_tag, Span::new(start, full_end))?);
        cursor = full_end;
    }

    if cursor < source.len() {
        nodes.push(Node::new(
            NodeKind::Raw(source[cursor..].to_string()),
            Span::new(cursor, source.len()),
        ));
    }

    Ok(TemplateAst { nodes })
}

fn parse_tag(raw_tag: &str, span: Span) -> Result<Node, TemplateEngineError> {
    let tag = raw_tag.trim();

    if tag.is_empty() {
        return Err(parse_error(ParseErrorKind::EmptyTag, span));
    }

    if let Some(name) = tag.strip_prefix(LITERAL_MARKER) {
        let name = parse_param_name(name.trim(), span)?;
        return Ok(Node::new(NodeKind::Literal(ParamRef::new(name)), span));
    }

    let name = parse_param_name(tag, span)?;
    Ok(Node::new(NodeKind::Display(ParamRef::new(name)), span))
}

fn parse_param_name(name: &str, span: Span) -> Result<&str, TemplateEngineError> {
    if name.is_empty() {
        return Err(parse_error(ParseErrorKind::EmptyParamName, span));
    }

    if !is_valid_param_name(name) {
        return Err(parse_error(ParseErrorKind::InvalidParamName, span));
    }

    Ok(name)
}

fn is_valid_param_name(name: &str) -> bool {
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn parse_error(kind: ParseErrorKind, span: Span) -> TemplateEngineError {
    TemplateEngineError::Parse { kind, span }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(value: &str, start: usize, end: usize) -> Node {
        Node::new(NodeKind::Raw(value.to_string()), Span::new(start, end))
    }

    fn display(name: &str, start: usize, end: usize) -> Node {
        Node::new(
            NodeKind::Display(ParamRef::new(name)),
            Span::new(start, end),
        )
    }

    fn literal(name: &str, start: usize, end: usize) -> Node {
        Node::new(
            NodeKind::Literal(ParamRef::new(name)),
            Span::new(start, end),
        )
    }

    #[test]
    fn parses_raw_text_only() {
        assert_eq!(
            parse_template("hello").unwrap(),
            TemplateAst {
                nodes: vec![raw("hello", 0, 5)]
            }
        );
    }

    #[test]
    fn parses_display_tag() {
        assert_eq!(
            parse_template("= {{ title }}").unwrap(),
            TemplateAst {
                nodes: vec![raw("= ", 0, 2), display("title", 2, 13)]
            }
        );
    }

    #[test]
    fn parses_literal_tag() {
        assert_eq!(
            parse_template("#let authors = {{= authors }}").unwrap(),
            TemplateAst {
                nodes: vec![raw("#let authors = ", 0, 15), literal("authors", 15, 29)]
            }
        );
    }

    #[test]
    fn parses_adjacent_tags() {
        assert_eq!(
            parse_template("{{first}}{{= second}}").unwrap(),
            TemplateAst {
                nodes: vec![display("first", 0, 9), literal("second", 9, 21)]
            }
        );
    }

    #[test]
    fn rejects_unclosed_tag() {
        let error = parse_template("{{ title").unwrap_err();

        assert!(matches!(
            error,
            TemplateEngineError::Parse {
                kind: ParseErrorKind::UnclosedTag,
                span: Span { start: 0, end: 8 }
            }
        ));
    }

    #[test]
    fn rejects_empty_tag() {
        let error = parse_template("{{ }}").unwrap_err();

        assert!(matches!(
            error,
            TemplateEngineError::Parse {
                kind: ParseErrorKind::EmptyTag,
                span: Span { start: 0, end: 5 }
            }
        ));
    }

    #[test]
    fn rejects_empty_literal_tag() {
        let error = parse_template("{{= }}").unwrap_err();

        assert!(matches!(
            error,
            TemplateEngineError::Parse {
                kind: ParseErrorKind::EmptyParamName,
                span: Span { start: 0, end: 6 }
            }
        ));
    }

    #[test]
    fn rejects_invalid_param_name() {
        let error = parse_template("{{ title name }}").unwrap_err();

        assert!(matches!(
            error,
            TemplateEngineError::Parse {
                kind: ParseErrorKind::InvalidParamName,
                span: Span { start: 0, end: 16 }
            }
        ));
    }

    #[test]
    fn rejects_trailing_literal_marker() {
        let error = parse_template("{{name =}}").unwrap_err();

        assert!(matches!(
            error,
            TemplateEngineError::Parse {
                kind: ParseErrorKind::InvalidParamName,
                span: Span { start: 0, end: 10 }
            }
        ));
    }
}
