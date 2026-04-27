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
    fn test_route_rejects_missing_prefix() {
        assert!(matches!(
            route_to_relative_path("tutorial/"),
            Err(DocsRenderError::MissingPrefix(_))
        ));
    }
}
