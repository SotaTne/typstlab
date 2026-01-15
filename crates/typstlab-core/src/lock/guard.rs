//! RAII lock guard for automatic lock release

use std::fs::File;
use std::path::PathBuf;

/// RAII guard for file locks
///
/// When this guard is dropped, the file lock is automatically released.
/// This ensures that locks are always released, even in the presence of
/// panics or early returns.
#[derive(Debug)]
pub struct LockGuard {
    #[allow(dead_code)]
    pub(crate) file: File,
    #[allow(dead_code)]
    pub(crate) path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Lock is automatically released when File is closed
        // fs2 advisory locks are released on file descriptor close
        // No explicit unlock call needed - RAII handles cleanup
    }
}
