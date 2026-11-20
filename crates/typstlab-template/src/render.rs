use super::ast::{NodeKind, TemplateAst};
use super::error::TemplateEngineError;
use super::parse::parse_template;
use super::value::Params;

/// Renders `{{ name }}` in Typst markup and `{{= name }}` as a Typst code literal.
pub fn render_text(source: &str, params: &Params) -> Result<String, TemplateEngineError> {
    let ast = parse_template(source)?;
    render_ast(&ast, params)
}

pub fn render_ast(ast: &TemplateAst, params: &Params) -> Result<String, TemplateEngineError> {
    let mut output = String::new();

    for node in &ast.nodes {
        match &node.kind {
            NodeKind::Raw(text) => output.push_str(text),
            NodeKind::Display(param_ref) => {
                let value = params
                    .get(&param_ref.name)
                    .ok_or_else(|| TemplateEngineError::MissingParam(param_ref.name.clone()))?;
                output.push_str(&value.to_typst_display()?);
            }
            NodeKind::Literal(param_ref) => {
                let value = params
                    .get(&param_ref.name)
                    .ok_or_else(|| TemplateEngineError::MissingParam(param_ref.name.clone()))?;
                output.push_str(&value.to_typst_literal()?);
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn renders_display_text() {
        let mut params = Params::new();
        params.insert(
            "title".to_string(),
            Value::String(r#"A "x" #panic() *b* // c"#.to_string()),
        );

        assert_eq!(
            render_text("= {{ title }}", &params).unwrap(),
            r#"= #("A \"x\" #panic() *b* // c")"#
        );
    }

    #[test]
    fn renders_typst_literal() {
        let mut params = Params::new();
        params.insert(
            "authors".to_string(),
            Value::Array(vec![
                Value::String("Sota".to_string()),
                Value::String("ChatGPT".to_string()),
            ]),
        );

        assert_eq!(
            render_text("#let authors = {{= authors }}", &params).unwrap(),
            r#"#let authors = ("Sota", "ChatGPT")"#
        );
    }

    #[test]
    fn renders_date_literal() {
        let mut params = Params::new();
        params.insert("date".to_string(), Value::Date(date(2026, 5, 27)));

        assert_eq!(
            render_text("#let date = {{= date }}", &params).unwrap(),
            "#let date = datetime(year: 2026, month: 5, day: 27)"
        );
    }

    #[test]
    fn rejects_missing_param() {
        assert!(matches!(
            render_text("= {{ title }}", &Params::new()).unwrap_err(),
            TemplateEngineError::MissingParam(name) if name == "title"
        ));
    }

    #[test]
    fn rejects_array_as_display_text() {
        let mut params = Params::new();
        params.insert("authors".to_string(), Value::Array(vec![]));

        assert!(matches!(
            render_text("{{ authors }}", &params).unwrap_err(),
            TemplateEngineError::NonDisplayableValue
        ));
    }
}
