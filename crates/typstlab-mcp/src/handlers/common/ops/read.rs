use crate::errors;

use super::safety::check_entry_safety;

/// Read a markdown file content with validation (blocking).
pub fn read_markdown_file_sync(
    path: &std::path::Path,
    project_root: &std::path::Path,
) -> Result<String, rmcp::ErrorData> {
    if !path.exists() || !path.is_file() {
        return Err(errors::resource_not_found(format!(
            "File not found: {}",
            path.display()
        )));
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Err(errors::invalid_input("File must be a markdown (.md) file"));
    }

    check_entry_safety(path, project_root)?;

    let metadata = std::fs::metadata(path).map_err(errors::from_display)?;
    if metadata.len() > typstlab_core::config::consts::search::MAX_FILE_BYTES {
        return Err(errors::file_too_large(format!(
            "File exceeds maximum allowed size of {} bytes",
            typstlab_core::config::consts::search::MAX_FILE_BYTES
        )));
    }

    std::fs::read_to_string(path).map_err(errors::from_display)
}
