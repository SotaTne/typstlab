//! Route to filepath conversion with security validation

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Converts DocsEntry route to file path
///
/// # Security
///
/// - Validates route after removing /DOCS-BASE/ prefix
/// - Blocks absolute paths
/// - Blocks parent directory traversal (..)
/// - Only allows relative paths under target_dir
///
/// # Mapping Rules
///
/// - "/DOCS-BASE/" → "index.md"
/// - "/DOCS-BASE/tutorial/" → "tutorial/index.md"
/// - "/DOCS-BASE/tutorial/writing/" → "tutorial/writing.md"
///
/// # Arguments
///
/// * `target_dir` - Base directory for generated files
/// * `route` - DocsEntry route (must start with "/DOCS-BASE/")
///
/// # Returns
///
/// Absolute path to the output markdown file
///
/// # Errors
///
/// Returns error if:
/// - Route doesn't start with "/DOCS-BASE/"
/// - Route contains absolute or rooted paths
/// - Route contains parent directory traversal (..)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use typstlab_typst::docs::generate::route_to_filepath;
///
/// let target = Path::new("/tmp/docs");
/// let path = route_to_filepath(target, "/DOCS-BASE/tutorial/writing/").unwrap();
/// assert_eq!(path, Path::new("/tmp/docs/tutorial/writing/index.md"));
/// ```
pub fn route_to_filepath(target_dir: &Path, route: &str) -> Result<PathBuf, RouteError> {
    // Remove /DOCS-BASE/ prefix
    let relative_route = strip_prefix(route)?;

    // Validate for security issues
    validate_route_security(relative_route)?;

    // Convert to path
    build_filepath(target_dir, relative_route)
}

/// Strip /DOCS-BASE/ prefix from route
fn strip_prefix(route: &str) -> Result<&str, RouteError> {
    route
        .strip_prefix("/DOCS-BASE/")
        .ok_or_else(|| RouteError::MissingPrefix(route.to_string()))
}

/// Validate route for security issues
fn validate_route_security(relative_route: &str) -> Result<(), RouteError> {
    if relative_route.is_empty() {
        return Ok(());
    }

    let route_path = Path::new(relative_route);

    // Check for absolute or rooted paths
    if typstlab_core::path::has_absolute_or_rooted_component(route_path) {
        return Err(RouteError::AbsolutePath(relative_route.to_string()));
    }

    // Check for parent directory traversal (..)
    if route_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(RouteError::PathTraversal(relative_route.to_string()));
    }

    Ok(())
}

/// Build filepath from validated route
fn build_filepath(target_dir: &Path, relative_route: &str) -> Result<PathBuf, RouteError> {
    let mut path = target_dir.to_path_buf();

    if relative_route.is_empty() {
        // Root: index.md
        path.push("index.md");
    } else if relative_route.ends_with('/') {
        // Directory: dir/index.md
        let dir_name = relative_route.trim_end_matches('/');
        path.push(dir_name);
        path.push("index.md");
    } else {
        // File: dir/file.md
        path.push(format!("{}.md", relative_route));
    }

    Ok(path)
}

/// Route conversion errors
#[derive(Debug, Error)]
pub enum RouteError {
    /// Route doesn't start with /DOCS-BASE/
    #[error("Route must start with /DOCS-BASE/: {0}")]
    MissingPrefix(String),

    /// Absolute or rooted path not allowed
    #[error("Absolute or rooted path not allowed: {0}")]
    AbsolutePath(String),

    /// Path traversal (..) not allowed
    #[error("Path traversal (..) not allowed: {0}")]
    PathTraversal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_to_filepath_root() {
        let target = Path::new("/tmp/docs");
        let path = route_to_filepath(target, "/DOCS-BASE/").expect("Should map root");
        assert_eq!(path, Path::new("/tmp/docs/index.md"));
    }

    #[test]
    fn test_route_to_filepath_directory() {
        let target = Path::new("/tmp/docs");
        let path = route_to_filepath(target, "/DOCS-BASE/tutorial/").expect("Should map directory");
        assert_eq!(path, Path::new("/tmp/docs/tutorial/index.md"));
    }

    #[test]
    fn test_route_to_filepath_file() {
        let target = Path::new("/tmp/docs");
        let path =
            route_to_filepath(target, "/DOCS-BASE/tutorial/writing").expect("Should map file");
        assert_eq!(path, Path::new("/tmp/docs/tutorial/writing.md"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let target = Path::new("/tmp/docs");
        let result = route_to_filepath(target, "/DOCS-BASE/../../../etc/passwd");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::PathTraversal(_)));
    }

    #[test]
    fn test_absolute_path_blocked() {
        let target = Path::new("/tmp/docs");
        let result = route_to_filepath(target, "/tmp/malicious");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::MissingPrefix(_)));
    }

    #[test]
    fn test_missing_prefix() {
        let target = Path::new("/tmp/docs");
        let result = route_to_filepath(target, "tutorial/writing");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::MissingPrefix(_)));
    }
}
