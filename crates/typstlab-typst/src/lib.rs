//! Typst binary resolution and command execution for typstlab.
//!
//! This crate provides functionality to resolve Typst binaries from multiple sources
//! and execute Typst commands with proper version validation.
//!
//! # Architecture
//!
//! The crate is organized into three main modules:
//!
//! - [`info`]: Core types representing binary metadata
//! - [`resolve`]: Binary resolution with multi-tier search strategy
//! - [`exec`]: Command execution with output capture
//!
//! # Binary Resolution Flow
//!
//! ```text
//! resolve_typst()
//!     ↓
//! 1. Check cache (if !force_refresh)
//!     ↓ (cache miss)
//! 2. Try managed cache
//!     → Search: {cache_dir}/{version}/typst
//!     → Validate version
//!     ↓ (not found)
//! 3. Try system PATH
//!     → Use which::which("typst")
//!     → Validate version
//!     ↓ (not found)
//! 4. Return NotFound with searched locations
//! ```
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use typstlab_typst::{resolve_typst, exec_typst, ResolveOptions, ExecOptions};
//! use std::path::PathBuf;
//!
//! # fn main() -> typstlab_core::Result<()> {
//! // Resolve Typst binary
//! let resolve_opts = ResolveOptions {
//!     required_version: "0.17.0".to_string(),
//!     project_root: PathBuf::from("."),
//!     force_refresh: false,
//! };
//!
//! let result = resolve_typst(resolve_opts)?;
//!
//! // Execute Typst command
//! let exec_opts = ExecOptions {
//!     project_root: PathBuf::from("."),
//!     args: vec!["compile".to_string(), "document.typ".to_string()],
//!     required_version: "0.17.0".to_string(),
//! };
//!
//! let exec_result = exec_typst(exec_opts)?;
//! println!("Exit code: {}", exec_result.exit_code);
//! # Ok(())
//! # }
//! ```
//!
//! ## Handling Resolution Results
//!
//! ```no_run
//! use typstlab_typst::{resolve_typst, ResolveOptions, ResolveResult};
//! use std::path::PathBuf;
//!
//! # fn main() -> typstlab_core::Result<()> {
//! let opts = ResolveOptions {
//!     required_version: "0.17.0".to_string(),
//!     project_root: PathBuf::from("."),
//!     force_refresh: false,
//! };
//!
//! match resolve_typst(opts)? {
//!     ResolveResult::Cached(info) => {
//!         println!("Found in cache: {:?}", info.path);
//!     }
//!     ResolveResult::Resolved(info) => {
//!         println!("Resolved from {}: {:?}", info.source, info.path);
//!     }
//!     ResolveResult::NotFound { required_version, searched_locations } => {
//!         println!("Version {} not found", required_version);
//!         println!("Searched: {:?}", searched_locations);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Managed Cache Structure
//!
//! Typst binaries are cached in OS-specific locations:
//!
//! - **macOS**: `~/Library/Caches/typstlab/typst/{version}/typst`
//! - **Linux**: `~/.cache/typstlab/typst/{version}/typst`
//! - **Windows**: `%LOCALAPPDATA%\typstlab\typst\{version}\typst.exe`

// Core modules
pub mod exec;
pub mod github;
pub mod info;
pub mod install;
pub mod resolve;

// Re-export commonly used types
pub use exec::{ExecOptions, ExecResult, exec_typst};
pub use info::{TypstInfo, TypstSource};
pub use resolve::{ResolveOptions, ResolveResult, resolve_typst};

// Type alias for convenience
pub type Result<T> = typstlab_core::Result<T>;
