//! Error types for file locking

use std::fmt;
use std::path::PathBuf;

/// Error type for lock operations
#[derive(Debug)]
pub enum LockError {
    /// Lock acquisition timed out
    Timeout {
        /// Path to the lock file
        path: PathBuf,
        /// Human-readable description
        description: String,
    },
    /// I/O error during lock operation
    Io {
        /// The underlying I/O error
        source: std::io::Error,
        /// Path to the lock file
        path: PathBuf,
        /// Operation that failed
        operation: String,
    },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::Timeout { path, description } => {
                write!(
                    f,
                    "Timeout waiting for lock on {} ({})",
                    path.display(),
                    description
                )
            }
            LockError::Io {
                source,
                path,
                operation,
            } => {
                write!(
                    f,
                    "I/O error during {} on {}: {}",
                    operation,
                    path.display(),
                    source
                )
            }
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockError::Timeout { .. } => None,
            LockError::Io { source, .. } => Some(source),
        }
    }
}
