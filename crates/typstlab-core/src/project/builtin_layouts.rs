//! Builtin layout definitions

use super::layout::Layout;

/// Get builtin layout by name
pub fn get_builtin_layout(name: &str) -> Option<Layout> {
    match name {
        "default" => Some(default_layout()),
        "minimal" => Some(minimal_layout()),
        _ => None,
    }
}

/// Default layout with full metadata and references
fn default_layout() -> Layout {
    Layout::new("default")
        .with_meta_template(include_str!("../../builtin_layouts/default/meta.tmp.typ"))
        .with_header_static(include_str!("../../builtin_layouts/default/header.typ"))
        .with_refs_template(include_str!("../../builtin_layouts/default/refs.tmp.typ"))
}

/// Minimal layout with basic metadata
fn minimal_layout() -> Layout {
    Layout::new("minimal")
        .with_meta_template(include_str!("../../builtin_layouts/minimal/meta.tmp.typ"))
        .with_refs_template(include_str!("../../builtin_layouts/minimal/refs.tmp.typ"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_default() {
        let layout = get_builtin_layout("default").unwrap();
        assert_eq!(layout.name, "default");
        assert!(layout.meta_template.is_some());
        assert!(layout.header_static.is_some());
        assert!(layout.refs_template.is_some());
    }

    #[test]
    fn test_get_builtin_minimal() {
        let layout = get_builtin_layout("minimal").unwrap();
        assert_eq!(layout.name, "minimal");
        assert!(layout.meta_template.is_some());
        assert!(layout.refs_template.is_some());
        // Minimal layout doesn't have header
        assert!(layout.header_static.is_none());
    }

    #[test]
    fn test_get_builtin_nonexistent() {
        let layout = get_builtin_layout("nonexistent");
        assert!(layout.is_none());
    }

    #[test]
    fn test_default_layout_has_each_loop() {
        let layout = default_layout();
        let meta = layout.meta_template.unwrap();
        // Should contain {{each}} or {{ each }} for authors
        assert!(meta.contains("each") && meta.contains("|author|"));
    }

    #[test]
    fn test_minimal_layout_simpler_than_default() {
        let default = default_layout();
        let minimal = minimal_layout();

        // Minimal should have shorter templates
        assert!(
            minimal.meta_template.as_ref().unwrap().len()
                < default.meta_template.as_ref().unwrap().len()
        );
    }
}
