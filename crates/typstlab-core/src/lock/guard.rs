//! RAII lock guard for automatic lock release

use std::fs::File;
use std::path::PathBuf;

/// RAII guard for file locks
///
/// When this guard is dropped, the file lock is automatically released.
/// This ensures that locks are always released, even in the presence of
/// panics or early returns.
pub struct LockGuard {
    #[allow(dead_code)]
    pub(crate) file: File,
    #[allow(dead_code)]
    pub(crate) path: PathBuf,
    /// Holds the process-level lock if applicable.
    /// Uses raw pointer/box trick to bypass lifetime issues with MutexGuard if needed,
    /// but here we'll try a simpler approach if possible.
    /// Since we can't easily store MutexGuard in a struct with the Mutex it came from,
    /// we just store it as a Box<dyn Any>.
    pub(crate) _process_guard: Option<Box<dyn std::any::Any + Send>>,
}

impl std::fmt::Debug for LockGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockGuard")
            .field("path", &self.path)
            .finish()
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // OS lock is automatically released when File is closed.
        // Process lock is released when _process_guard is dropped.
    }
}

// Safety: MutexGuard is !Send, so if we store it, LockGuard becomes !Send.
// That's generally OK for file locks which are thread-bound.
// However, if we need it to be Send, we have to be careful.
// For now, let's assume !Send is acceptable.
unsafe impl Send for LockGuard {} 
