//! Function signature formatting for Markdown output
//!
//! Generates code-formatted signature strings like:
//! - `func(param: type) -> return_type`
//! - `array.map(function: function) -> array`

use crate::docs::schema::FuncContent;

/// Formats function signature: `name(param: type, ...) -> return_type`
///
/// Creates a code-formatted signature string showing:
/// - Function path (if present): `array.map(...)`
/// - Parameter names with types
/// - Default values for optional parameters
/// - Return type(s)
///
/// # Arguments
///
/// * `func` - Function content from docs.json
///
/// # Returns
///
/// Formatted signature string with backticks (e.g., `` `func(x: int) -> int` ``)
///
/// # Examples
///
/// ```
/// use typstlab_typst::docs::schema::FuncContent;
/// use typstlab_typst::docs::render::format_function_signature;
///
/// let func = FuncContent {
///     path: vec!["array".to_string()],
///     name: "map".to_string(),
///     title: "Map".to_string(),
///     keywords: vec![],
///     oneliner: None,
///     element: false,
///     contextual: false,
///     details: None,
///     example: None,
///     is_self: false,
///     params: vec![],
///     returns: vec!["array".to_string()],
///     scope: vec![],
/// };
///
/// assert_eq!(format_function_signature(&func), "`array.map() -> array`");
/// ```
pub fn format_function_signature(func: &FuncContent) -> String {
    let mut sig = String::new();

    // Function name (with path if present)
    if !func.path.is_empty() {
        sig.push_str(&format!("`{}.{}(", func.path.join("."), func.name));
    } else {
        sig.push_str(&format!("`{}(", func.name));
    }

    // Parameters
    let param_strs: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let mut ps = p.name.clone();

            // Add type annotation
            if !p.types.is_empty() {
                ps.push_str(": ");
                ps.push_str(&p.types.join(" | "));
            }

            // Add default value
            if let Some(default) = &p.default {
                ps.push_str(" = ");
                ps.push_str(&serde_json::to_string(default).unwrap_or_else(|_| "?".to_string()));
            }

            ps
        })
        .collect();

    sig.push_str(&param_strs.join(", "));
    sig.push(')');

    // Return type
    if !func.returns.is_empty() {
        sig.push_str(" -> ");
        sig.push_str(&func.returns.join(" | "));
    }

    sig.push('`');
    sig
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::schema::ParamContent;
    use serde_json::json;

    #[test]
    fn test_simple_signature() {
        let func = FuncContent {
            path: vec![],
            name: "assert".to_string(),
            title: "Assert".to_string(),
            keywords: vec![],
            oneliner: None,
            element: false,
            contextual: false,
            details: None,
            example: None,
            is_self: false,
            params: vec![],
            returns: vec!["none".to_string()],
            scope: vec![],
        };

        assert_eq!(format_function_signature(&func), "`assert() -> none`");
    }

    #[test]
    fn test_signature_with_path() {
        let func = FuncContent {
            path: vec!["array".to_string()],
            name: "map".to_string(),
            title: "Map".to_string(),
            keywords: vec![],
            oneliner: None,
            element: false,
            contextual: false,
            details: None,
            example: None,
            is_self: false,
            params: vec![],
            returns: vec!["array".to_string()],
            scope: vec![],
        };

        assert_eq!(format_function_signature(&func), "`array.map() -> array`");
    }

    #[test]
    fn test_signature_with_params() {
        let func = FuncContent {
            path: vec![],
            name: "func".to_string(),
            title: "Func".to_string(),
            keywords: vec![],
            oneliner: None,
            element: false,
            contextual: false,
            details: None,
            example: None,
            is_self: false,
            params: vec![
                ParamContent {
                    name: "x".to_string(),
                    details: None,
                    example: None,
                    types: vec!["int".to_string()],
                    strings: vec![],
                    default: None,
                    positional: true,
                    named: false,
                    required: true,
                    variadic: false,
                    settable: false,
                },
                ParamContent {
                    name: "y".to_string(),
                    details: None,
                    example: None,
                    types: vec!["int".to_string(), "float".to_string()],
                    strings: vec![],
                    default: Some(json!(42)),
                    positional: true,
                    named: false,
                    required: false,
                    variadic: false,
                    settable: false,
                },
            ],
            returns: vec!["int".to_string()],
            scope: vec![],
        };

        assert_eq!(
            format_function_signature(&func),
            "`func(x: int, y: int | float = 42) -> int`"
        );
    }

    #[test]
    fn test_signature_no_return() {
        let func = FuncContent {
            path: vec![],
            name: "func".to_string(),
            title: "Func".to_string(),
            keywords: vec![],
            oneliner: None,
            element: false,
            contextual: false,
            details: None,
            example: None,
            is_self: false,
            params: vec![],
            returns: vec![],
            scope: vec![],
        };

        assert_eq!(format_function_signature(&func), "`func()`");
    }
}
