pub fn html_to_markdown(html: &str) -> String {
    let mut markdown = String::with_capacity(html.len());
    let mut tag = String::new();
    let mut inside_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                inside_tag = true;
                tag.clear();
            }
            '>' if inside_tag => {
                inside_tag = false;
                apply_tag_boundary(&tag, &mut markdown);
            }
            _ if inside_tag => tag.push(ch),
            _ => markdown.push(ch),
        }
    }

    decode_entities(markdown.trim()).trim().to_string()
}

pub fn details_to_markdown(details: &serde_json::Value) -> String {
    let html = extract_html_from_details(details);
    if html.is_empty() {
        String::new()
    } else {
        html_to_markdown(&html)
    }
}

pub fn extract_html_from_details(details: &serde_json::Value) -> String {
    match details {
        serde_json::Value::String(html) => html.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(extract_html_item)
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}

pub fn value_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => {
            if text.contains('<') && text.contains('>') {
                html_to_markdown(text)
            } else {
                decode_entities(text)
            }
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn extract_html_item(item: &serde_json::Value) -> Option<String> {
    let object = item.as_object()?;
    if object.get("kind")?.as_str()? != "html" {
        return None;
    }
    object.get("content")?.as_str().map(ToOwned::to_owned)
}

fn apply_tag_boundary(tag: &str, output: &mut String) {
    let name = tag
        .trim()
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("");

    match name {
        "p" | "div" | "section" | "ul" | "ol" | "table" => push_blank_line(output),
        "br" | "li" | "tr" => push_newline(output),
        "h1" => push_heading(output, "# "),
        "h2" => push_heading(output, "## "),
        "h3" => push_heading(output, "### "),
        "code" => output.push('`'),
        _ => {}
    }
}

fn push_heading(output: &mut String, prefix: &str) {
    push_blank_line(output);
    output.push_str(prefix);
}

fn push_blank_line(output: &mut String) {
    if !output.ends_with("\n\n") {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
}

fn push_newline(output: &mut String) {
    if !output.ends_with('\n') {
        output.push('\n');
    }
}

fn decode_entities(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_keeps_text_and_code() {
        let markdown = html_to_markdown("<p>Use <code>assert</code>.</p>");

        assert!(markdown.contains("Use `assert`."));
    }

    #[test]
    fn test_extract_html_from_details_array() {
        let details = serde_json::json!([
            {"kind": "html", "content": "<p>First</p>"},
            {"kind": "example", "content": "ignored"},
            {"kind": "html", "content": "<p>Second</p>"}
        ]);

        assert_eq!(
            extract_html_from_details(&details),
            "<p>First</p>\n\n<p>Second</p>"
        );
    }

    #[test]
    fn test_value_to_text_decodes_html_entities() {
        let value = serde_json::json!("&lt;code&gt;auto&lt;/code&gt;");

        assert_eq!(value_to_text(&value), "<code>auto</code>");
    }
}
