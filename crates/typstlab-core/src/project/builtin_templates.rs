//! Builtin template definitions

use super::template::Template;

/// Get builtin template by name
pub fn get_builtin_template(name: &str) -> Option<Template> {
    match name {
        "default" => Some(default_template()),
        _ => None,
    }
}

/// Default template with main.tmp.typ and template.typ
fn default_template() -> Template {
    Template::new("default")
        .with_file(
            "main.tmp.typ",
            include_str!("../../builtin_templates/default/main.tmp.typ"),
        )
        .with_file(
            "template.typ",
            include_str!("../../builtin_templates/default/template.typ"),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_default() {
        let template = get_builtin_template("default").unwrap();
        assert_eq!(template.theme, "default");
        assert_eq!(template.files.len(), 2);
    }

    #[test]
    fn test_get_builtin_nonexistent() {
        let template = get_builtin_template("minimal");
        assert!(template.is_none());
    }
}
