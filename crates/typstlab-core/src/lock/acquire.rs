//! Lock acquisition logic with retry and timeout

use super::{LockError, LockGuard};
use fs2::FileExt;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(10);
const MAX_RETRY_DELAY: Duration = Duration::from_millis(500);
const PROGRESS_MESSAGE_THRESHOLD: Duration = Duration::from_secs(2);

/// Attempts to acquire an exclusive lock with retry and timeout
pub(crate) fn acquire_with_retry(
    lock_path: &Path,
    timeout: Duration,
    description: &str,
) -> Result<LockGuard, LockError> {
    // Create parent directories if needed
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).map_err(|e| LockError::Io {
            source: e,
            path: lock_path.to_path_buf(),
            operation: "create parent directories".to_string(),
        })?;
    }

    let start = Instant::now();
    let mut retry_delay = INITIAL_RETRY_DELAY;
    let mut progress_shown = false;

    loop {
        // Try to open/create lock file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| LockError::Io {
                source: e,
                path: lock_path.to_path_buf(),
                operation: "open lock file".to_string(),
            })?;

        // Try to acquire exclusive lock (non-blocking)
        match file.try_lock_exclusive() {
            Ok(()) => {
                // Successfully acquired lock
                return Ok(LockGuard {
                    file,
                    path: lock_path.to_path_buf(),
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Lock is held by another process, retry
                let elapsed = start.elapsed();

                // Check timeout
                if elapsed >= timeout {
                    return Err(LockError::Timeout {
                        path: lock_path.to_path_buf(),
                        description: description.to_string(),
                    });
                }

                // Show progress message after threshold
                if !progress_shown && elapsed >= PROGRESS_MESSAGE_THRESHOLD {
                    eprintln!(
                        "Waiting for lock on {} ({})...",
                        lock_path.display(),
                        description
                    );
                    progress_shown = true;
                }

                // Sleep before retry
                thread::sleep(retry_delay);

                // Exponential backoff
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
            }
            Err(e) => {
                // Other I/O error
                return Err(LockError::Io {
                    source: e,
                    path: lock_path.to_path_buf(),
                    operation: "acquire lock".to_string(),
                });
            }
        }
    }
}
