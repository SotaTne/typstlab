//! Tests for file locking module

use super::{acquire_lock, acquire_shared_lock, LockError};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_acquire_lock_success() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("test.lock");

    // Should successfully acquire lock
    let lock_guard = acquire_lock(&lock_path, Duration::from_secs(5), "test lock");
    assert!(lock_guard.is_ok(), "Should acquire lock successfully");

    // Lock should exist
    assert!(lock_path.exists(), "Lock file should exist");
}

#[test]
fn test_lock_blocks_concurrent_access() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("concurrent.lock");

    let lock_path_clone = lock_path.clone();
    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = barrier.clone();

    // Thread 1: Acquire lock and hold it
    let handle1 = thread::spawn(move || {
        let _lock = acquire_lock(&lock_path_clone, Duration::from_secs(5), "thread1").unwrap();
        barrier_clone.wait(); // Signal that lock is acquired
        thread::sleep(Duration::from_millis(300)); // Hold lock longer than thread2's timeout
                                                   // Lock released when _lock drops
    });

    // Wait for thread 1 to acquire lock
    barrier.wait();

    // Thread 2: Try to acquire same lock with short timeout
    // Note: 200ms timeout (increased from 50ms) provides more reliability on CI systems
    // where scheduling jitter can be significant
    let start = std::time::Instant::now();
    let result = acquire_lock(&lock_path, Duration::from_millis(200), "thread2");

    // Should timeout because thread 1 holds the lock
    assert!(result.is_err(), "Should fail to acquire lock");
    assert!(
        matches!(result, Err(LockError::Timeout { .. })),
        "Should be a timeout error"
    );

    // Should have waited for the timeout duration
    assert!(
        start.elapsed() >= Duration::from_millis(200),
        "Should wait for timeout"
    );

    handle1.join().unwrap();
}

#[test]
fn test_lock_timeout_after_duration() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("timeout.lock");

    // Acquire lock first
    let _lock1 = acquire_lock(&lock_path, Duration::from_secs(5), "first").unwrap();

    // Try to acquire again with 100ms timeout
    let start = std::time::Instant::now();
    let result = acquire_lock(&lock_path, Duration::from_millis(100), "second");

    // Should fail with timeout
    assert!(
        result.is_err(),
        "Second lock acquisition should fail with timeout"
    );
    assert!(
        matches!(result, Err(LockError::Timeout { .. })),
        "Error should be Timeout variant"
    );

    // Should have waited approximately the timeout duration
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(100) && elapsed < Duration::from_millis(200),
        "Should timeout after approximately 100ms, got {:?}",
        elapsed
    );
}

#[test]
fn test_lock_released_on_drop() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("drop.lock");

    // Acquire and immediately drop
    {
        let _lock = acquire_lock(&lock_path, Duration::from_secs(5), "test").unwrap();
        // Lock held here
    } // Lock should be released here when _lock goes out of scope

    // Should be able to acquire again immediately
    let result = acquire_lock(&lock_path, Duration::from_millis(50), "test2");
    assert!(
        result.is_ok(),
        "Should be able to acquire lock after drop, got {:?}",
        result
    );
}

#[test]
fn test_lock_retry_with_progress() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("retry.lock");

    let lock_path_clone = lock_path.clone();

    // Thread 1: Hold lock for 150ms
    let handle = thread::spawn(move || {
        let _lock = acquire_lock(&lock_path_clone, Duration::from_secs(5), "holder").unwrap();
        thread::sleep(Duration::from_millis(150));
        // Lock released on drop
    });

    // Wait a bit to ensure thread 1 acquires lock first
    thread::sleep(Duration::from_millis(10));

    // Thread 2: Try to acquire with 1 second timeout (should succeed after thread 1 releases)
    let start = std::time::Instant::now();
    let result = acquire_lock(&lock_path, Duration::from_secs(1), "waiter");

    // Should succeed after waiting
    assert!(result.is_ok(), "Should acquire lock after retry");

    // Should have waited for thread 1 to release
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(100) && elapsed < Duration::from_millis(500),
        "Should acquire after waiting, elapsed: {:?}",
        elapsed
    );

    handle.join().unwrap();
}

#[test]
fn test_lock_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("nested/dir/test.lock");

    // Parent directories don't exist yet
    assert!(!lock_path.parent().unwrap().exists());

    // Should create parent directories automatically
    let result = acquire_lock(&lock_path, Duration::from_secs(5), "test");
    assert!(
        result.is_ok(),
        "Should create parent directories and acquire lock"
    );

    // Parent directory should now exist
    assert!(lock_path.parent().unwrap().exists());
}

#[test]
fn test_lock_description_used_in_error() {
    let temp_dir = TempDir::new().unwrap();
    let lock_path = temp_dir.path().join("desc.lock");

    // Acquire lock first
    let _lock1 = acquire_lock(&lock_path, Duration::from_secs(5), "operation foo").unwrap();

    // Try to acquire again with descriptive name
    let result = acquire_lock(&lock_path, Duration::from_millis(50), "operation bar");

    // Error should contain the description
    assert!(result.is_err());
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("operation bar") || error_msg.contains(&lock_path.display().to_string()),
        "Error message should contain description or path: {}",
        error_msg
    );
}

#[test]
fn test_multiple_different_locks() {
    let temp_dir = TempDir::new().unwrap();
    let lock1_path = temp_dir.path().join("lock1.lock");
    let lock2_path = temp_dir.path().join("lock2.lock");

    // Should be able to acquire different locks simultaneously
    let _lock1 = acquire_lock(&lock1_path, Duration::from_secs(5), "lock1").unwrap();
    let _lock2 = acquire_lock(&lock2_path, Duration::from_secs(5), "lock2").unwrap();

    // Both should be held
    assert!(lock1_path.exists());
    assert!(lock2_path.exists());
}

// ============================================================================
// RED Phase Tests for Phase 2.9: Shared Locking (Windows CI Fix)
// ============================================================================
//
// These tests verify that multiple readers can hold shared locks simultaneously,
// which is necessary to fix the Windows CI ERROR_ACCESS_DENIED issue where
// concurrent readers block writers during State::save().

#[test]
fn test_shared_lock_allows_concurrent_readers() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    const NUM_READERS: usize = 5;
    let barrier = Arc::new(Barrier::new(NUM_READERS));
    let success_count = Arc::new(AtomicUsize::new(0));

    // Spawn 5 reader threads that acquire shared locks simultaneously
    let handles: Vec<_> = (0..NUM_READERS)
        .map(|_| {
            let lock_path = lock_path.clone();
            let barrier = Arc::clone(&barrier);
            let success_count = Arc::clone(&success_count);

            thread::spawn(move || {
                barrier.wait(); // Synchronize start

                // All threads should acquire shared locks without blocking
                let _guard =
                    acquire_shared_lock(&lock_path, Duration::from_secs(1), "concurrent read test")
                        .unwrap();

                success_count.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(50)); // Hold lock briefly
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify: All readers succeeded (no blocking)
    assert_eq!(
        success_count.load(Ordering::SeqCst),
        NUM_READERS,
        "All {} readers should acquire shared locks concurrently",
        NUM_READERS
    );
}

#[test]
fn test_shared_lock_blocks_exclusive_writer() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    let barrier = Arc::new(Barrier::new(2));
    let writer_blocked = Arc::new(AtomicBool::new(false));

    // Reader thread: Acquire shared lock and hold
    let lock_path_reader = lock_path.clone();
    let barrier_reader = Arc::clone(&barrier);
    let reader_handle = thread::spawn(move || {
        let _guard =
            acquire_shared_lock(&lock_path_reader, Duration::from_secs(1), "reader").unwrap();

        barrier_reader.wait(); // Signal: shared lock acquired
        thread::sleep(Duration::from_millis(300)); // Hold lock (increased from 200ms for CI stability)
    });

    // Writer thread: Try to acquire exclusive lock (should block)
    let lock_path_writer = lock_path.clone();
    let barrier_writer = Arc::clone(&barrier);
    let writer_blocked_clone = Arc::clone(&writer_blocked);
    let writer_handle = thread::spawn(move || {
        barrier_writer.wait(); // Wait for reader to acquire lock

        // Try to acquire exclusive lock (should timeout because reader holds shared lock)
        let result = acquire_lock(&lock_path_writer, Duration::from_millis(150), "writer");

        if result.is_err() {
            writer_blocked_clone.store(true, Ordering::SeqCst);
        }
    });

    reader_handle.join().unwrap();
    writer_handle.join().unwrap();

    // Verify: Writer was blocked by shared lock
    assert!(
        writer_blocked.load(Ordering::SeqCst),
        "Exclusive lock should be blocked by shared lock"
    );
}

#[test]
fn test_exclusive_lock_blocks_shared_readers() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    let barrier = Arc::new(Barrier::new(2));
    let reader_blocked = Arc::new(AtomicBool::new(false));

    // Writer thread: Acquire exclusive lock and hold
    let lock_path_writer = lock_path.clone();
    let barrier_writer = Arc::clone(&barrier);
    let writer_handle = thread::spawn(move || {
        let _guard = acquire_lock(&lock_path_writer, Duration::from_secs(1), "writer").unwrap();

        barrier_writer.wait(); // Signal: exclusive lock acquired
        thread::sleep(Duration::from_millis(300)); // Hold lock (increased from 200ms for CI stability)
    });

    // Reader thread: Try to acquire shared lock (should block)
    let lock_path_reader = lock_path.clone();
    let barrier_reader = Arc::clone(&barrier);
    let reader_blocked_clone = Arc::clone(&reader_blocked);
    let reader_handle = thread::spawn(move || {
        barrier_reader.wait(); // Wait for writer to acquire lock

        // Try to acquire shared lock (should timeout because writer holds exclusive lock)
        let result = acquire_shared_lock(&lock_path_reader, Duration::from_millis(150), "reader");

        if result.is_err() {
            reader_blocked_clone.store(true, Ordering::SeqCst);
        }
    });

    writer_handle.join().unwrap();
    reader_handle.join().unwrap();

    // Verify: Reader was blocked by exclusive lock
    assert!(
        reader_blocked.load(Ordering::SeqCst),
        "Shared lock should be blocked by exclusive lock"
    );
}
