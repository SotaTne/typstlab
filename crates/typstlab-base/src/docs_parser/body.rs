use super::error::DocsRenderError;
use super::html::{details_to_markdown, html_to_markdown, value_to_text};
use super::schema::DocsBody;
use super::schema::{
    CategoryContent, FuncContent, GroupContent, ParamContent, SymbolsContent, TypeContent,
};

pub fn body_to_markdown(body: &DocsBody) -> Result<String, DocsRenderError> {
    match body.kind.as_str() {
        "html" => render_html_body(&body.content),
        "func" => render_func_body(&body.content),
        "type" => render_type_body(&body.content),
        "category" => render_category_body(&body.content),
        "group" => render_group_body(&body.content),
        "symbols" => render_symbols_body(&body.content),
        unknown => Ok(format!("<!-- Unknown body kind: {} -->", unknown)),
    }
}

fn render_html_body(content: &serde_json::Value) -> Result<String, DocsRenderError> {
    let html = content
        .as_str()
        .ok_or_else(|| DocsRenderError::Body("html body content must be a string".to_string()))?;
    Ok(html_to_markdown(html))
}

fn render_func_body(content: &serde_json::Value) -> Result<String, DocsRenderError> {
    let func: FuncContent = serde_json::from_value(content.clone())?;
    Ok(render_func(&func))
}

fn render_type_body(content: &serde_json::Value) -> Result<String, DocsRenderError> {
    let type_content: TypeContent = serde_json::from_value(content.clone())?;
    let mut markdown = String::new();

    if let Some(details) = &type_content.details {
        push_section_text(&mut markdown, &details_to_markdown(details));
    }

    if let Some(constructor) = &type_content.constructor {
        markdown.push_str("## Constructor\n\n");
        markdown.push_str(&render_func(constructor));
    }

    if !type_content.scope.is_empty() {
        markdown.push_str("## Methods\n\n");
        for method in &type_content.scope {
            markdown.push_str(&render_scoped_func(method));
        }
    }

    Ok(markdown)
}

fn render_category_body(content: &serde_json::Value) -> Result<String, DocsRenderError> {
    let category: CategoryContent = serde_json::from_value(content.clone())?;
    let mut markdown = String::new();

    if let Some(details) = &category.details {
        push_section_text(&mut markdown, &details_to_markdown(details));
    }

    if !category.items.is_empty() {
        markdown.push_str("## Items\n\n");
        for item in &category.items {
            markdown.push_str(&format!("- [{}]({})", item.name, docs_link(&item.route)));
            if let Some(oneliner) = &item.oneliner {
                markdown.push_str(" - ");
                markdown.push_str(oneliner);
            }
            markdown.push('\n');
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn render_group_body(content: &serde_json::Value) -> Result<String, DocsRenderError> {
    let group: GroupContent = serde_json::from_value(content.clone())?;
    let mut markdown = String::new();

    if let Some(details) = &group.details {
        push_section_text(&mut markdown, &details_to_markdown(details));
    }

    if !group.functions.is_empty() {
        markdown.push_str("## Functions\n\n");
        for func in &group.functions {
            let route = if func.path.is_empty() {
                format!("{}.md", func.name)
            } else {
                format!("{}/{}.md", func.path.join("/"), func.name)
            };
            markdown.push_str(&format!("- [{}]({})", func.title, route));
            if let Some(oneliner) = &func.oneliner {
                markdown.push_str(" - ");
                markdown.push_str(oneliner);
            }
            markdown.push('\n');
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn render_symbols_body(content: &serde_json::Value) -> Result<String, DocsRenderError> {
    let symbols: SymbolsContent = serde_json::from_value(content.clone())?;
    let mut markdown = String::new();

    if let Some(details) = &symbols.details {
        push_section_text(&mut markdown, &details_to_markdown(details));
    }

    if !symbols.list.is_empty() {
        markdown.push_str("## Symbols\n\n");
        markdown.push_str("| Name | Markup | Math | Unicode |\n");
        markdown.push_str("|------|--------|------|---------|\n");
        for symbol in &symbols.list {
            let markup = code_or_dash(symbol.markup_shorthand.as_deref());
            let math = code_or_dash(symbol.math_shorthand.as_deref());
            let unicode = symbol_unicode(symbol.value.as_deref(), symbol.codepoint);
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                symbol.name, markup, math, unicode
            ));
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn render_func(func: &FuncContent) -> String {
    let mut markdown = String::new();
    markdown.push_str("## Signature\n\n");
    markdown.push_str(&format_signature(func));
    markdown.push_str("\n\n");

    if let Some(oneliner) = &func.oneliner {
        markdown.push_str(oneliner);
        markdown.push_str("\n\n");
    }

    if let Some(details) = &func.details {
        push_section_text(&mut markdown, &details_to_markdown(details));
    }

    if let Some(example) = &func.example {
        let example = details_to_markdown(example);
        if !example.is_empty() {
            markdown.push_str("## Example\n\n");
            markdown.push_str(&example);
            markdown.push_str("\n\n");
        }
    }

    if !func.params.is_empty() {
        markdown.push_str("## Parameters\n\n");
        for param in &func.params {
            markdown.push_str(&format_parameter(param));
        }
        markdown.push('\n');
    }

    if !func.returns.is_empty() {
        markdown.push_str("## Returns\n\n");
        markdown.push_str(&format!("`{}`\n\n", func.returns.join(" | ")));
    }

    if !func.scope.is_empty() {
        markdown.push_str("## Methods\n\n");
        for method in &func.scope {
            markdown.push_str(&render_scoped_func(method));
        }
    }

    markdown
}

fn render_scoped_func(func: &FuncContent) -> String {
    let mut markdown = String::new();
    markdown.push_str(&format!("### `{}`\n\n", func.name));
    markdown.push_str(&format_signature(func));
    markdown.push_str("\n\n");
    if let Some(oneliner) = &func.oneliner {
        markdown.push_str(oneliner);
        markdown.push_str("\n\n");
    }
    if let Some(details) = &func.details {
        push_section_text(&mut markdown, &details_to_markdown(details));
    }
    markdown
}

fn format_signature(func: &FuncContent) -> String {
    let params = func
        .params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    if func.returns.is_empty() {
        format!("`{}({})`", func.name, params)
    } else {
        format!(
            "`{}({}) -> {}`",
            func.name,
            params,
            func.returns.join(" | ")
        )
    }
}

fn format_parameter(param: &ParamContent) -> String {
    let mut markdown = String::new();
    markdown.push_str(&format!("- **{}**", param.name));

    if !param.types.is_empty() {
        markdown.push_str(&format!(" (`{}`)", param.types.join(" | ")));
    }

    let flags = parameter_flags(param);
    if !flags.is_empty() {
        markdown.push_str(", ");
        markdown.push_str(&flags.join(", "));
    }

    if let Some(default) = &param.default {
        markdown.push_str(", default: `");
        markdown.push_str(&value_to_text(default));
        markdown.push('`');
    }

    markdown.push_str(":\n");
    if let Some(details) = &param.details {
        let details = details_to_markdown(details);
        for line in details.lines() {
            markdown.push_str("  ");
            markdown.push_str(line);
            markdown.push('\n');
        }
    }
    markdown.push('\n');
    markdown
}

fn parameter_flags(param: &ParamContent) -> Vec<&'static str> {
    let mut flags = Vec::new();
    flags.push(if param.required {
        "required"
    } else {
        "optional"
    });
    if param.positional {
        flags.push("positional");
    }
    if param.named {
        flags.push("named");
    }
    if param.variadic {
        flags.push("variadic");
    }
    if param.settable {
        flags.push("settable");
    }
    flags
}

fn push_section_text(markdown: &mut String, text: &str) {
    if !text.is_empty() {
        markdown.push_str(text);
        markdown.push_str("\n\n");
    }
}

fn docs_link(route: &str) -> String {
    route
        .strip_prefix("/DOCS-BASE/")
        .unwrap_or(route)
        .trim_end_matches('/')
        .to_string()
        + ".md"
}

fn code_or_dash(value: Option<&str>) -> String {
    value
        .map(|value| format!("`{}`", value))
        .unwrap_or_else(|| "-".to_string())
}

fn symbol_unicode(value: Option<&str>, codepoint: Option<u32>) -> String {
    if let Some(codepoint) = codepoint {
        return format!("U+{:04X}", codepoint);
    }
    value
        .and_then(|value| value.chars().next())
        .map(|ch| format!("U+{:04X}", ch as u32))
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs_parser::schema::DocsBody;

    #[test]
    fn test_unknown_body_kind_is_rendered_as_comment() {
        let body = DocsBody {
            kind: "future".to_string(),
            content: serde_json::json!({}),
        };

        let markdown = body_to_markdown(&body).unwrap();

        assert!(markdown.contains("Unknown body kind: future"));
    }

    #[test]
    fn test_render_func_body() {
        let body = DocsBody {
            kind: "func".to_string(),
            content: serde_json::json!({
                "name": "assert",
                "title": "Assert",
                "oneliner": "Ensures a condition is true.",
                "params": [
                    {
                        "name": "condition",
                        "types": ["bool"],
                        "required": true,
                        "positional": true
                    }
                ],
                "returns": ["none"]
            }),
        };

        let markdown = body_to_markdown(&body).unwrap();

        assert!(markdown.contains("## Signature"));
        assert!(markdown.contains("`assert("));
        assert!(markdown.contains("**condition**"));
        assert!(markdown.contains("## Returns"));
    }

    #[test]
    fn test_render_type_category_group_symbols_bodies() {
        let type_body = DocsBody {
            kind: "type".to_string(),
            content: serde_json::json!({
                "name": "Array",
                "title": "Array",
                "constructor": {
                    "name": "array",
                    "title": "Array",
                    "params": [],
                    "returns": ["array"]
                },
                "scope": [
                    {
                        "name": "push",
                        "title": "Push",
                        "oneliner": "Appends a value.",
                        "params": [
                            {"name": "value", "types": ["any"], "required": true, "positional": true}
                        ],
                        "returns": ["none"]
                    }
                ]
            }),
        };
        let category_body = DocsBody {
            kind: "category".to_string(),
            content: serde_json::json!({
                "name": "foundations",
                "title": "Foundations",
                "items": [
                    {"name": "foo", "route": "/DOCS-BASE/reference/foo/", "oneliner": "Foo."}
                ]
            }),
        };
        let group_body = DocsBody {
            kind: "group".to_string(),
            content: serde_json::json!({
                "name": "calc",
                "title": "Calc",
                "functions": [
                    {"name": "add", "title": "Add", "oneliner": "Add numbers.", "params": [], "returns": ["none"]}
                ]
            }),
        };
        let symbols_body = DocsBody {
            kind: "symbols".to_string(),
            content: serde_json::json!({
                "name": "emoji",
                "title": "Emoji",
                "list": [
                    {
                        "name": "smile",
                        "value": "😊",
                        "markupShorthand": ":)",
                        "mathShorthand": "\\smile"
                    }
                ]
            }),
        };

        let type_markdown = body_to_markdown(&type_body).unwrap();
        let category_markdown = body_to_markdown(&category_body).unwrap();
        let group_markdown = body_to_markdown(&group_body).unwrap();
        let symbols_markdown = body_to_markdown(&symbols_body).unwrap();

        assert!(type_markdown.contains("## Constructor"));
        assert!(type_markdown.contains("## Methods"));
        assert!(category_markdown.contains("## Items"));
        assert!(group_markdown.contains("## Functions"));
        assert!(symbols_markdown.contains("## Symbols"));
        assert!(symbols_markdown.contains("| smile | `:)` | `\\smile` | U+1F60A |"));
    }
}
