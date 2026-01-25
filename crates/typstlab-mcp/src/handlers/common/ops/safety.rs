use crate::errors;
use std::path::Path;

/// Check if a path entry is safe to process.
/// `path`: The path to check
/// `project_root`: The root of the project to ensure no escape
pub fn check_entry_safety(path: &Path, project_root: &Path) -> Result<(), rmcp::ErrorData> {
    // 1. Symlink check (Strictly forbidden)
    // Note: If path is project_root/rules (symlink), we should probably allow it IF it points inside project?
    // User requirement: "rules/docs のルート自体がシンボリックリンクでも許容される... プロジェクト外へのエスケープを完全に防げていない"
    // So strictly forbid symlinks even for roots if they point outside?
    // Actually typically symlinks are forbidden entirely in this project design to avoid complexity.
    let metadata = std::fs::symlink_metadata(path).map_err(|e| {
        rmcp::ErrorData::internal_error(format!("Failed to read metadata: {}", e), None)
    })?;

    if metadata.is_symlink() {
        return Err(errors::path_escape("Symlinks are not allowed"));
    }

    // 2. Canonicalize check (Path escape)
    let canonical_path = std::fs::canonicalize(path).map_err(|e| {
        // If file doesn't exist, canonicalize fails. Check existence before?
        // Caller usually ensures existence or we handle error.
        rmcp::ErrorData::internal_error(format!("Canonicalize failed: {}", e), None)
    })?;

    let canonical_root = std::fs::canonicalize(project_root).map_err(|e| {
        rmcp::ErrorData::internal_error(format!("Canonicalize project root failed: {}", e), None)
    })?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(errors::path_escape("Path escapes project root"));
    }

    Ok(())
}
