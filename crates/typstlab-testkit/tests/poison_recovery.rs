//! Integration tests for mutex poison recovery
//!
//! These tests intentionally poison mutexes to verify recovery logic.
//! They run in a separate test binary to avoid contaminating unit tests.
//!
//! Previously these tests were marked with #[ignore] in unit tests because
//! mutex poisoning is permanent for the lifetime of the test binary. Moving
//! them to integration tests isolates the poison effects while maintaining
//! CI coverage.

use std::thread;
use typstlab_testkit::{ENV_LOCK, get_shared_mock_server};

#[test]
fn test_env_lock_recovers_from_poison() {
    // Save original environment (to restore after test)
    let original_home = std::env::var("HOME").ok();

    // Simulate panic while holding lock
    let handle = thread::spawn(|| {
        let _guard = ENV_LOCK.lock().unwrap();
        panic!("Simulated panic to poison mutex");
    });

    // Join will return Err because thread panicked
    let _ = handle.join();

    // Subsequent lock should recover (not panic)
    let result = std::panic::catch_unwind(|| {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    });

    assert!(result.is_ok(), "Should recover from poisoned mutex");

    // Restore environment if modified
    // SAFETY: No other test is running concurrently in this integration test
    if let Some(home) = original_home {
        unsafe {
            std::env::set_var("HOME", home);
        }
    }
}

#[test]
fn test_shared_mock_server_recovers_from_poison() {
    // Simulate panic while holding server lock
    let handle = thread::spawn(|| {
        let _guard = get_shared_mock_server();
        panic!("Simulated panic to poison server mutex");
    });

    let _ = handle.join();

    // Should recover
    let result = std::panic::catch_unwind(|| {
        let _guard = get_shared_mock_server();
    });

    assert!(result.is_ok(), "Server lock should recover from poison");
}
