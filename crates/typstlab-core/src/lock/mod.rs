//! File locking module for process-level mutual exclusion
//!
//! This module provides advisory file locks using the fs2 crate to prevent
//! race conditions when multiple processes access shared resources.

use std::path::Path;
use std::time::Duration;

mod acquire;
mod error;
mod guard;

pub use error::LockError;
pub use guard::LockGuard;

#[cfg(test)]
mod tests;

/// Acquires an exclusive lock on the specified path with a timeout.
///
/// This function attempts to acquire an exclusive (write) lock on the specified
/// file path. If the lock cannot be acquired immediately, it will retry with
/// exponential backoff until the timeout is reached.
///
/// # Arguments
///
/// * `lock_path` - The path where the lock file will be created
/// * `timeout` - Maximum duration to wait for lock acquisition
/// * `description` - Human-readable description for progress messages
///
/// # Returns
///
/// Returns a `LockGuard` on success, which will automatically release the lock
/// when dropped. Returns `LockError` on timeout or I/O error.
///
/// # Examples
///
/// ```no_run
/// use typstlab_core::lock::acquire_lock;
/// use std::time::Duration;
/// use std::path::Path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let lock_path = Path::new("/tmp/my.lock");
/// let guard = acquire_lock(lock_path, Duration::from_secs(30), "my operation")?;
/// // Critical section here
/// drop(guard); // Explicit drop (automatic on scope exit)
/// # Ok(())
/// # }
/// ```
pub fn acquire_lock(
    lock_path: &Path,
    timeout: Duration,
    description: &str,
) -> Result<LockGuard, LockError> {
    acquire::acquire_with_retry(lock_path, timeout, description)
}

/// Acquires a shared lock on the specified path with a timeout.
///
/// This function attempts to acquire a shared (read) lock on the specified
/// file path. Multiple threads/processes can hold shared locks simultaneously,
/// but shared locks conflict with exclusive locks.
///
/// # Use Cases
///
/// - Reading state.json (multiple concurrent readers allowed)
/// - Shared resources where writes are rare
/// - Prevents torn reads (reader never sees partial state)
///
/// # Arguments
///
/// * `lock_path` - The path where the lock file will be created
/// * `timeout` - Maximum duration to wait for lock acquisition
/// * `description` - Human-readable description for progress messages
///
/// # Returns
///
/// Returns a `LockGuard` on success, which will automatically release the lock
/// when dropped. Returns `LockError` on timeout or I/O error.
///
/// # Examples
///
/// ```no_run
/// use typstlab_core::lock::acquire_shared_lock;
/// use std::time::Duration;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let lock_path = std::env::temp_dir().join("my.lock");
/// let guard = acquire_shared_lock(&lock_path, Duration::from_secs(5), "read state")?;
/// // Critical section (read-only)
/// drop(guard); // Explicit drop (automatic on scope exit)
/// # Ok(())
/// # }
/// ```
pub fn acquire_shared_lock(
    lock_path: &Path,
    timeout: Duration,
    description: &str,
) -> Result<LockGuard, LockError> {
    acquire::acquire_shared_with_retry(lock_path, timeout, description)
}
