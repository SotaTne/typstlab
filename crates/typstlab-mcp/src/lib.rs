// Re-export rmcp for convenience
pub use rmcp;

// Export tools module
pub mod server;
pub mod tools;

// Re-export the 4 MCP tools with rmcp_tool macro applied
pub use tools::rules::{rules_get, rules_list, rules_page, rules_search};

// Re-export input/output types for external use
pub use tools::rules::{
    FileEntry, RulesGetInput, RulesGetOutput, RulesListInput, RulesListOutput, RulesPageInput,
    RulesPageOutput, RulesScope, RulesSearchInput, RulesSearchOutput, RulesSubdir, SearchMatch,
};
