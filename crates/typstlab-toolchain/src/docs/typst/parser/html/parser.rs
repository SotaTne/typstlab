use super::ast::{HtmlAttrs, HtmlElement, HtmlNode, HtmlTag, HtmlTree};
use super::decode_entities;
use super::error::HtmlParseError;
use html5gum::emitters::default::{StartTag, Token};
use html5gum::{HtmlString, Spanned, Tokenizer};

#[derive(Debug)]
struct Frame {
    tag: Option<HtmlTag>,
    attrs: HtmlAttrs,
    children: Vec<HtmlNode>,
}

impl Frame {
    fn root() -> Self {
        Self {
            tag: None,
            attrs: HtmlAttrs::default(),
            children: Vec::new(),
        }
    }

    fn element(tag: HtmlTag, attrs: HtmlAttrs) -> Self {
        Self {
            tag: Some(tag),
            attrs,
            children: Vec::new(),
        }
    }

    fn into_node(self) -> Option<HtmlNode> {
        self.tag.map(|tag| {
            HtmlNode::Element(HtmlElement {
                tag,
                attrs: self.attrs,
                children: self.children,
            })
        })
    }
}

pub fn parse_html(input: &str) -> Result<HtmlTree, HtmlParseError> {
    let mut stack = vec![Frame::root()];
    let mut skip_depth = 0usize;

    for token in Tokenizer::new(input) {
        match token.map_err(|error| HtmlParseError::Tokenizer(error.to_string()))? {
            Token::StartTag(tag) => handle_start_tag(tag, &mut stack, &mut skip_depth)?,
            Token::EndTag(tag) => {
                let name = html_string_to_string(&tag.name).to_ascii_lowercase();
                handle_end_tag(&name, &mut stack, &mut skip_depth)?;
            }
            Token::String(text) => {
                if skip_depth == 0 {
                    push_text(&mut stack, &html_string_to_string(&text.value))?;
                }
            }
            Token::Comment(_) | Token::Doctype(_) => {}
            Token::Error(error) => {
                return Err(HtmlParseError::Tokenizer(format!("{:?}", error.value)));
            }
        }
    }

    while stack.len() > 1 {
        close_top_frame(&mut stack)?;
    }

    let root = stack.pop().ok_or(HtmlParseError::EmptyStack)?;
    Ok(HtmlTree {
        children: root.children,
    })
}

fn handle_start_tag(
    tag: StartTag<()>,
    stack: &mut Vec<Frame>,
    skip_depth: &mut usize,
) -> Result<(), HtmlParseError> {
    let name = html_string_to_string(&tag.name).to_ascii_lowercase();

    if *skip_depth > 0 {
        if !tag.self_closing {
            *skip_depth += 1;
        }
        return Ok(());
    }

    if HtmlTag::is_skipped_name(&name) {
        if !tag.self_closing {
            *skip_depth = 1;
        }
        return Ok(());
    }

    let html_tag = HtmlTag::from_name(&name).ok_or(HtmlParseError::UnsupportedTag(name))?;
    let attrs = attrs_from_start_tag(&tag);

    if html_tag.is_void() || tag.self_closing {
        current_frame(stack)?
            .children
            .push(HtmlNode::Element(HtmlElement {
                tag: html_tag,
                attrs,
                children: Vec::new(),
            }));
    } else {
        stack.push(Frame::element(html_tag, attrs));
    }

    Ok(())
}

fn handle_end_tag(
    name: &str,
    stack: &mut Vec<Frame>,
    skip_depth: &mut usize,
) -> Result<(), HtmlParseError> {
    if *skip_depth > 0 {
        *skip_depth -= 1;
        return Ok(());
    }

    let Some(tag) = HtmlTag::from_name(name) else {
        return Ok(());
    };

    while stack.len() > 1 {
        let is_target = stack.last().and_then(|frame| frame.tag) == Some(tag);
        close_top_frame(stack)?;
        if is_target {
            break;
        }
    }

    Ok(())
}

fn push_text(stack: &mut [Frame], text: &str) -> Result<(), HtmlParseError> {
    let text = decode_entities(text);
    if preserve_text(stack) || !text.trim().is_empty() {
        stack
            .last_mut()
            .ok_or(HtmlParseError::EmptyStack)?
            .children
            .push(HtmlNode::Text(text));
    }
    Ok(())
}

fn preserve_text(stack: &[Frame]) -> bool {
    stack
        .iter()
        .any(|frame| matches!(frame.tag, Some(HtmlTag::Pre | HtmlTag::Code)))
}

fn close_top_frame(stack: &mut Vec<Frame>) -> Result<(), HtmlParseError> {
    let frame = stack.pop().ok_or(HtmlParseError::EmptyStack)?;
    if let Some(node) = frame.into_node() {
        current_frame(stack)?.children.push(node);
    }
    Ok(())
}

fn current_frame(stack: &mut [Frame]) -> Result<&mut Frame, HtmlParseError> {
    stack.last_mut().ok_or(HtmlParseError::EmptyStack)
}

fn attrs_from_start_tag(tag: &StartTag<()>) -> HtmlAttrs {
    let mut attrs = HtmlAttrs::default();

    for (name, value) in &tag.attributes {
        let name = html_string_to_string(name).to_ascii_lowercase();
        let value = spanned_html_string_to_string(value);
        let decoded = decode_entities(&value);
        match name.as_str() {
            "href" => attrs.href = Some(decoded),
            "src" => attrs.src = Some(decoded),
            "alt" => attrs.alt = Some(decoded),
            "title" => attrs.title = Some(decoded),
            "class" => attrs.class = Some(decoded),
            "id" => attrs.id = Some(decoded),
            _ => {}
        }
    }

    attrs
}

fn spanned_html_string_to_string(value: &Spanned<HtmlString, ()>) -> String {
    html_string_to_string(&value.value)
}

fn html_string_to_string(value: &HtmlString) -> String {
    String::from_utf8_lossy(value.as_slice()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tmp_docs_tags() {
        let tree = parse_html(
            r#"<h2>Title</h2><p>A <strong>bold</strong> <a href="/x">link</a>.</p><pre><code>x</code></pre><table><thead><tr><th>A</th></tr></thead><tbody><tr><td>B</td></tr></tbody></table>"#,
        )
        .unwrap();

        assert!(!tree.children.is_empty());
    }

    #[test]
    fn parses_details_tags() {
        let tree = parse_html("<details><summary>More</summary><p>Body</p></details>").unwrap();

        assert!(!tree.children.is_empty());
    }

    #[test]
    fn preserves_whitespace_inside_code() {
        let tree = parse_html(
            r#"<pre><code>// Don't do this
<span>#text</span>(
  size: <span>16pt</span>,
)[Heading]
</code></pre>"#,
        )
        .unwrap();

        let HtmlNode::Element(pre) = &tree.children[0] else {
            panic!("expected pre element");
        };
        assert!(
            super::super::markdown::html_tree_to_markdown_document(&tree)
                .unwrap()
                .to_string()
                .contains("// Don't do this\n#text(\n  size: 16pt,\n)[Heading]")
        );
        assert_eq!(pre.tag, HtmlTag::Pre);
    }

    #[test]
    fn rejects_unknown_tags() {
        let error = parse_html("<custom-tag>text</custom-tag>").unwrap_err();

        assert!(matches!(error, HtmlParseError::UnsupportedTag(_)));
    }
}
