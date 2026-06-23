use std::path::{Component, Path, PathBuf};

use super::DocsRenderError;

const DOCS_ROUTE_PREFIX: &str = "/DOCS-BASE/";

pub fn route_to_relative_path(route: &str) -> Result<PathBuf, DocsRenderError> {
    if route.is_empty() {
        return Err(DocsRenderError::EmptyRoute);
    }

    let relative = route
        .strip_prefix(DOCS_ROUTE_PREFIX)
        .ok_or_else(|| DocsRenderError::MissingPrefix(route.to_string()))?;

    validate_relative_path_text(relative)?;

    if relative.is_empty() {
        return Ok(PathBuf::from("index.md"));
    }

    let trimmed = relative.trim_end_matches('/');
    Ok(Path::new(trimmed).with_extension("md"))
}

pub fn route_to_relative_link(
    source_route: &str,
    target_route: &str,
) -> Result<PathBuf, DocsRenderError> {
    let source_path = route_to_relative_path(source_route)?;
    let target_path = route_to_relative_path(target_route)?;
    Ok(relative_path_between(&source_path, &target_path))
}

pub fn resolve_docs_href(source_route: &str, href: &str) -> Result<String, DocsRenderError> {
    let Some(target) = href.strip_prefix(DOCS_ROUTE_PREFIX) else {
        return Ok(href.to_string());
    };

    let (target_route, fragment) = match target.split_once('#') {
        Some((route, fragment)) => (route, Some(fragment)),
        None => (target, None),
    };

    let target_route = format!("{DOCS_ROUTE_PREFIX}{target_route}");
    let mut relative = markdown_path_string(&route_to_relative_link(source_route, &target_route)?);
    if let Some(fragment) = fragment {
        relative.push('#');
        relative.push_str(fragment);
    }

    Ok(relative)
}

pub fn validate_output_path(path: &Path) -> Result<(), DocsRenderError> {
    let path_text = path.to_string_lossy();
    validate_relative_path_text(&path_text)
}

fn validate_relative_path_text(path: &str) -> Result<(), DocsRenderError> {
    if path.starts_with('/') || path.starts_with('\\') || has_windows_drive_prefix(path) {
        return Err(DocsRenderError::RootedPath(path.to_string()));
    }

    for component in Path::new(path).components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(DocsRenderError::RootedPath(path.to_string()));
            }
            Component::ParentDir => return Err(DocsRenderError::PathTraversal(path.to_string())),
            _ => {}
        }
    }

    Ok(())
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn relative_path_between(from: &Path, to: &Path) -> PathBuf {
    let from_dir = from.parent().unwrap_or_else(|| Path::new(""));
    let from_components = normalized_components(from_dir);
    let to_components = normalized_components(to);

    let mut common = 0;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    let mut relative = PathBuf::new();
    for _ in common..from_components.len() {
        relative.push("..");
    }

    for component in &to_components[common..] {
        relative.push(component);
    }

    if relative.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        relative
    }
}

fn normalized_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

pub fn markdown_path_string(path: &Path) -> String {
    let mut text = String::new();
    for (index, component) in path.components().enumerate() {
        if index > 0 {
            text.push('/');
        }
        text.push_str(&component.as_os_str().to_string_lossy());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_root_maps_to_index() {
        assert_eq!(
            route_to_relative_path("/DOCS-BASE/").unwrap(),
            Path::new("index.md")
        );
    }

    #[test]
    fn test_route_nested_maps_to_markdown_path() {
        assert_eq!(
            route_to_relative_path("/DOCS-BASE/tutorial/writing/").unwrap(),
            PathBuf::from("tutorial").join("writing.md")
        );
    }

    #[test]
    fn test_route_relative_link_from_same_directory() {
        assert_eq!(
            route_to_relative_link(
                "/DOCS-BASE/reference/text/",
                "/DOCS-BASE/reference/text/highlight/"
            )
            .unwrap(),
            PathBuf::from("text").join("highlight.md")
        );
    }

    #[test]
    fn test_markdown_path_string_uses_forward_slashes() {
        assert_eq!(
            markdown_path_string(&PathBuf::from("text").join("highlight.md")),
            "text/highlight.md"
        );
    }

    #[test]
    fn test_route_relative_link_from_root_to_nested() {
        assert_eq!(
            route_to_relative_link("/DOCS-BASE/", "/DOCS-BASE/tutorial/writing/").unwrap(),
            PathBuf::from("tutorial").join("writing.md")
        );
    }

    #[test]
    fn test_route_rejects_missing_prefix() {
        assert!(matches!(
            route_to_relative_path("tutorial/"),
            Err(DocsRenderError::MissingPrefix(_))
        ));
    }
}
