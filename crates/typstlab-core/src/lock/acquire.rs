//! Lock acquisition logic with retry and timeout

use super::{LockError, LockGuard};
use fs2::FileExt;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(10);
const MAX_RETRY_DELAY: Duration = Duration::from_millis(500);
const PROGRESS_MESSAGE_THRESHOLD: Duration = Duration::from_secs(2);

/// Global registry of in-process locks to prevent threads from same process
/// entering critical section for same file simultaneously.
static PROCESS_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

fn get_process_locks() -> &'static Mutex<HashMap<PathBuf, Arc<Mutex<()>>>> {
    PROCESS_LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Helper to bypass MutexGuard !Send for LockGuard
struct SendableMutexGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
}
unsafe impl Send for SendableMutexGuard {}

/// Check if an I/O error indicates the lock is held and should be retried
fn should_retry_lock(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(target_os = "windows")]
    if error.raw_os_error() == Some(33) {
        return true;
    }
    false
}

/// Attempts to acquire an exclusive lock with retry and timeout
pub(crate) fn acquire_with_retry(
    lock_path: &Path,
    timeout: Duration,
    description: &str,
) -> Result<LockGuard, LockError> {
    // 1. Acquire process-level lock
    let abs_path = if lock_path.exists() {
        fs::canonicalize(lock_path).unwrap_or_else(|_| lock_path.to_path_buf())
    } else {
        let parent = lock_path.parent().unwrap_or(Path::new("."));
        let abs_parent = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
        abs_parent.join(lock_path.file_name().unwrap_or_default())
    };
    
    let process_mutex = {
        let mut registry = get_process_locks().lock().unwrap();
        registry
            .entry(abs_path.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };

    let start = Instant::now();
    let process_guard = loop {
        match process_mutex.try_lock() {
            Ok(guard) => {
                let static_guard = unsafe {
                    std::mem::transmute::<std::sync::MutexGuard<'_, ()>, std::sync::MutexGuard<'static, ()>>(guard)
                };
                break Box::new(SendableMutexGuard { _guard: static_guard });
            },
            Err(_) => {
                if start.elapsed() >= timeout {
                    return Err(LockError::Timeout {
                        path: lock_path.to_path_buf(),
                        description: description.to_string(),
                    });
                }
                thread::sleep(INITIAL_RETRY_DELAY);
            }
        }
    };

    // 2. Acquire OS-level lock
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let mut retry_delay = INITIAL_RETRY_DELAY;
    let mut progress_shown = false;

    loop {
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

        match file.try_lock_exclusive() {
            Ok(()) => {
                return Ok(LockGuard {
                    file,
                    path: lock_path.to_path_buf(),
                    _process_guard: Some(process_guard),
                });
            }
            Err(e) if should_retry_lock(&e) => {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    return Err(LockError::Timeout {
                        path: lock_path.to_path_buf(),
                        description: description.to_string(),
                    });
                }
                if !progress_shown && elapsed >= PROGRESS_MESSAGE_THRESHOLD {
                    eprintln!("Waiting for lock on {} ({})...", lock_path.display(), description);
                    progress_shown = true;
                }
                let remaining = timeout.saturating_sub(elapsed);
                thread::sleep(retry_delay.min(remaining));
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
            }
            Err(e) => {
                return Err(LockError::Io {
                    source: e,
                    path: lock_path.to_path_buf(),
                    operation: "acquire lock".to_string(),
                });
            }
        }
    }
}

/// Attempts to acquire a shared lock with retry and timeout
pub(crate) fn acquire_shared_with_retry(
    lock_path: &Path,
    timeout: Duration,
    description: &str,
) -> Result<LockGuard, LockError> {
    let start = Instant::now();
    let mut retry_delay = INITIAL_RETRY_DELAY;
    let mut progress_shown = false;

    loop {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| LockError::Io {
                source: e,
                path: lock_path.to_path_buf(),
                operation: "open lock file".to_string(),
            })?;

        match file.try_lock_shared() {
            Ok(()) => {
                return Ok(LockGuard {
                    file,
                    path: lock_path.to_path_buf(),
                    _process_guard: None,
                });
            }
            Err(e) => {
                let io_err: std::io::Error = e.into();
                if should_retry_lock(&io_err) {
                    let elapsed = start.elapsed();
                    if elapsed >= timeout {
                        return Err(LockError::Timeout {
                            path: lock_path.to_path_buf(),
                            description: description.to_string(),
                        });
                    }
                    if !progress_shown && elapsed >= PROGRESS_MESSAGE_THRESHOLD {
                        eprintln!("Waiting for shared lock on {} ({})...", lock_path.display(), description);
                        progress_shown = true;
                    }
                    let remaining = timeout.saturating_sub(elapsed);
                    thread::sleep(retry_delay.min(remaining));
                    retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                    continue;
                } else {
                    return Err(LockError::Io {
                        source: io_err,
                        path: lock_path.to_path_buf(),
                        operation: "acquire shared lock".to_string(),
                    });
                }
            }
        }
    }
}
