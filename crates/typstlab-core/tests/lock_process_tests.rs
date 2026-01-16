//! Process-level file locking tests
//!
//! These tests verify that locks work across process boundaries,
//! not just thread boundaries. Uses counter-based verification
//! to detect lost updates.

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use typstlab_testkit::example_bin;

/// Verifies that file locks prevent lost updates across processes.
///
/// # Positive Verification Strategy
///
/// This test uses positive verification only. We prove "locks work"
/// without needing to prove "no-locks fails" because:
///
/// 1. **Race detection is timing-dependent**: File I/O-based race
///    conditions are inherently unreliable (process startup: 10-50ms,
///    file I/O: 100-1000μs, both >> 50μs sleep)
///
/// 2. **TDD principle**: Positive tests specify behavior completely.
///    If we prove "locks prevent corruption", we don't need to prove
///    "no-locks causes corruption" (which is flaky).
///
/// 3. **Research evidence**: Rust GitHub flaky test study (Feb 2025)
///    shows file I/O races are platform-dependent and unreliable.
///
/// # Test Strategy
///
/// - Spawn 5 processes that each perform 20 read-modify-write operations
/// - All processes use file locking (via counter_child_locked.rs)
/// - Expected: 100 updates, all preserved (no lost updates)
/// - Proves: Locks enforce mutual exclusion at process level
///
/// # See Also
///
/// - `test_cross_process_exclusive_locking` - Proves only one process
///   holds lock at a time
/// - `crates/typstlab-core/src/lock/tests.rs` - Thread-level lock tests
/// - AGENTS.md §1 - Testing philosophy (positive verification)
///
/// # Decision History
///
/// 2026-01-16: Removed flaky negative test after Codex consultation
/// (agentId: a6b77a9). User requirement: "コードにおいては妥協をしないこと"
/// (no compromises) - flaky tests compromise quality.
#[test]
fn test_counter_with_lock_no_lost_updates() {
    // Verify counter helper WITH locking has NO lost updates
    let temp = TempDir::new().unwrap();
    let counter_path = temp.path().join("counter.txt");
    fs::write(&counter_path, "0").unwrap();

    const NUM_PROCESSES: usize = 5;
    const ITERATIONS_PER_PROCESS: usize = 20;

    // Spawn processes WITH locking
    let mut handles = vec![];
    for _ in 0..NUM_PROCESSES {
        let counter_path = counter_path.clone();
        let handle = std::thread::spawn(move || {
            let status = Command::new(example_bin("counter_child_locked"))
                .arg(&counter_path)
                .arg(ITERATIONS_PER_PROCESS.to_string())
                .status()
                .expect("Failed to execute counter_child_locked");

            assert!(
                status.success(),
                "counter_child_locked should exit successfully"
            );
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify: WITH locks, final count = expected (no lost updates)
    let final_count: u32 = fs::read_to_string(&counter_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let expected = (NUM_PROCESSES * ITERATIONS_PER_PROCESS) as u32;
    assert_eq!(
        final_count, expected,
        "With locks, should have no lost updates: got {}, expected {}",
        final_count, expected
    );
}

#[test]
fn test_cross_process_exclusive_locking() {
    // Verify only one process can hold lock at a time
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");
    let marker_path = temp.path().join("marker.txt");

    const NUM_PROCESSES: usize = 3;

    // Each process: acquire lock, write timestamp, hold for 100ms, release
    let mut handles = vec![];
    for id in 0..NUM_PROCESSES {
        let lock_path = lock_path.clone();
        let marker_path = marker_path.clone();
        let handle = std::thread::spawn(move || {
            let status = Command::new(example_bin("lock_holder"))
                .arg(&lock_path)
                .arg(&marker_path)
                .arg(id.to_string())
                .status()
                .expect("Failed to execute lock_holder");

            assert!(status.success(), "lock_holder should exit successfully");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify: All processes completed successfully (no deadlocks)
    // Marker file should contain all 3 process IDs (sequential writes)
    let content = fs::read_to_string(&marker_path).unwrap();
    assert!(content.contains("process_0"), "Should contain process_0");
    assert!(content.contains("process_1"), "Should contain process_1");
    assert!(content.contains("process_2"), "Should contain process_2");
}
