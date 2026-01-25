//! 共通パス解決とセキュリティチェック

use crate::errors;
use rmcp::ErrorData as McpError;
use std::path::{Component, Path, PathBuf};

/// 安全なパス解決を行う
///
/// 以下のセキュリティチェックを実施：
/// 1. 絶対パス/ルートパスの拒否
/// 2. 親ディレクトリ参照 (`..`) の拒否
/// 3. サンドボックス検証（canonicalize後にルート配下にあることを確認）
///
/// # Arguments
///
/// * `root` - ベースとなるルートディレクトリ（例: `.typstlab/kb/typst/docs`）
/// * `relative` - rootからの相対パス（例: `subdir/file.md`）
///
/// # Returns
///
/// `root.join(relative)` の結果（存在確認とセキュリティチェック済み）
///
/// # Errors
///
/// - 絶対パスやルートパスが含まれる場合
/// - `..` が含まれる場合
/// - 存在するパスがcanonicalizeした際にroot外を指している場合
pub async fn resolve_safe_path(root: &Path, requested: &Path) -> Result<PathBuf, McpError> {
    // Safety: この関数は特権的操作ではなく、セキュリティチェックを行うだけ
    // （root外へのアクセスは禁止するが、root内へのアクセスは呼び出し元が判断）

    // Reject absolute or rooted paths immediately (before canonicalization)
    if typstlab_core::path::has_absolute_or_rooted_component(requested) {
        return Err(errors::path_escape("Path cannot be absolute or rooted"));
    }

    // Reject paths containing ".." components (path traversal attempt)
    // This is a security check to prevent escaping the root directory
    if requested
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(errors::path_escape("Path cannot contain .."));
    }

    // Join paths
    let full_path = root.join(requested);

    // If the path exists, perform canonicalization and sandbox check
    if full_path.exists() {
        let canonical_target = tokio::fs::canonicalize(&full_path)
            .await
            .map_err(errors::from_display)?;
        let canonical_root = tokio::fs::canonicalize(root)
            .await
            .map_err(errors::from_display)?;

        if !canonical_target.starts_with(canonical_root) {
            return Err(errors::path_escape(format!(
                "Path '{}' resolves outside allowed root",
                requested.display()
            )));
        }
    }

    Ok(full_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_resolve_safe_path_accepts_valid_relative() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("subdir")).await.unwrap();
        fs::write(root.join("subdir/file.txt"), "content")
            .await
            .unwrap();

        let result = resolve_safe_path(root, Path::new("subdir/file.txt")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), root.join("subdir/file.txt"));
    }

    #[tokio::test]
    async fn test_resolve_safe_path_rejects_absolute() {
        let temp = TempDir::new().unwrap();
        let result = resolve_safe_path(temp.path(), Path::new("/etc/passwd")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("absolute or rooted"));
    }

    #[tokio::test]
    async fn test_resolve_safe_path_rejects_parent_traversal() {
        let temp = TempDir::new().unwrap();
        let result = resolve_safe_path(temp.path(), Path::new("../etc")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".."));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_resolve_safe_path_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        fs::create_dir_all(&root).await.unwrap();

        let outside = TempDir::new().unwrap();
        let outside_file = outside.path().join("secret.txt");
        fs::write(&outside_file, "secret").await.unwrap();

        let link = root.join("link.txt");
        symlink(&outside_file, &link).unwrap();

        let result = resolve_safe_path(&root, Path::new("link.txt")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("outside"));
    }

    #[tokio::test]
    async fn test_resolve_safe_path_accepts_nonexistent() {
        let temp = TempDir::new().unwrap();
        let result = resolve_safe_path(temp.path(), Path::new("nonexistent.txt")).await;
        // 存在しないパスは許容される（作成前のチェックなど）
        assert!(result.is_ok());
    }
}
