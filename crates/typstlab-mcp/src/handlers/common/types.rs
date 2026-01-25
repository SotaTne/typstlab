/// Common configuration for search operations
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum number of files to scan
    pub max_files: usize,
    /// Maximum number of matches to return
    pub max_matches: usize,
    /// File extensions to include (e.g., ["md", "typ"])
    pub file_extensions: Vec<String>,
}

impl SearchConfig {
    /// Create a new SearchConfig with default values
    pub fn new(max_files: usize, max_matches: usize, file_extensions: Vec<String>) -> Self {
        Self {
            max_files,
            max_matches,
            file_extensions,
        }
    }
}

/// Result of a directory browse operation
#[derive(Debug, serde::Serialize)]
pub struct BrowseResult {
    pub missing: bool,
    pub items: Vec<BrowseItem>,
    pub truncated: bool,
}

/// An item in a directory listing
#[derive(Debug, serde::Serialize)]
pub struct BrowseItem {
    pub name: String,
    pub path: String, // Relative path from project root
    #[serde(rename = "type")]
    pub item_type: String,
}

/// Result of a search operation
#[derive(Debug, serde::Serialize)]
pub struct SearchResult {
    pub matches: Vec<serde_json::Value>,
    pub truncated: bool,
    #[serde(skip)]
    // Internal field for debugging/logging; not serialized to JSON
    pub scanned_files: usize,
}
