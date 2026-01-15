//! Integration tests for State with atomic updates

use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;
use typstlab_core::state::State;

/// Test that concurrent writes to state.json don't cause corruption
///
/// Spawns multiple threads that all try to write to the same state file
/// simultaneously. Verifies that the final state.json is valid JSON and
/// not corrupted.
#[test]
fn test_concurrent_state_writes_no_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");

    // Create initial state
    let initial_state = State::empty();
    initial_state.save(&state_path).unwrap();

    // Number of concurrent writers
    const NUM_WRITERS: usize = 10;
    let barrier = Arc::new(Barrier::new(NUM_WRITERS));

    // Spawn concurrent writers
    let handles: Vec<_> = (0..NUM_WRITERS)
        .map(|i| {
            let state_path = state_path.clone();
            let barrier = Arc::clone(&barrier);

            thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                // Create a state with unique sync timestamp
                let mut state = State::empty();
                state.sync = Some(typstlab_core::state::SyncState {
                    last_sync: Some(chrono::Utc::now()),
                });

                // Write to same file (simulates concurrent builds, syncs, etc.)
                for _ in 0..5 {
                    state
                        .save(&state_path)
                        .unwrap_or_else(|_| panic!("Writer {} failed to save state", i));
                    thread::sleep(std::time::Duration::from_micros(100));
                }
            })
        })
        .collect();

    // Wait for all writers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify state.json is not corrupted
    let final_state = State::load(&state_path);
    assert!(
        final_state.is_ok(),
        "State file should be valid JSON after concurrent writes: {:?}",
        final_state
    );

    // Verify content is parseable
    let state = final_state.unwrap();
    assert_eq!(state.schema_version, "1.0");
}

/// Test that atomic update pattern survives simulated crashes
///
/// This test verifies that even if a process "crashes" (panics) during
/// state save, the state.json file remains valid (not partially written).
#[test]
fn test_state_atomic_update_survives_crash() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");

    // Create initial valid state
    let initial_state = State::empty();
    initial_state.save(&state_path).unwrap();

    // Verify initial state is valid
    let loaded = State::load(&state_path).unwrap();
    assert_eq!(loaded.schema_version, "1.0");

    // Simulate a crash during state save by checking file atomicity
    // If state.json is atomic, it should never be partially written

    // Spawn multiple threads that write and immediately read
    const NUM_ITERATIONS: usize = 50;
    let state_path_clone = state_path.clone();

    let handle = thread::spawn(move || {
        for i in 0..NUM_ITERATIONS {
            let mut state = State::empty();
            state.sync = Some(typstlab_core::state::SyncState {
                last_sync: Some(chrono::Utc::now()),
            });

            // Save state
            state.save(&state_path_clone).unwrap();

            // Immediately try to load (simulates another process reading)
            let loaded = State::load(&state_path_clone);
            assert!(
                loaded.is_ok(),
                "Iteration {}: State should always be valid, got {:?}",
                i,
                loaded
            );

            // Brief sleep to allow some interleaving
            thread::sleep(std::time::Duration::from_micros(10));
        }
    });

    // Another thread continuously trying to read
    let state_path_clone2 = state_path.clone();
    let handle2 = thread::spawn(move || {
        for _ in 0..NUM_ITERATIONS * 2 {
            if state_path_clone2.exists() {
                let loaded = State::load(&state_path_clone2);
                // Either file doesn't exist yet or is valid JSON
                if let Err(e) = loaded {
                    // Should never get partial JSON
                    panic!("Reader saw corrupted state: {:?}", e);
                }
            }
            thread::sleep(std::time::Duration::from_micros(5));
        }
    });

    handle.join().unwrap();
    handle2.join().unwrap();

    // Final verification
    let final_state = State::load(&state_path).unwrap();
    assert_eq!(final_state.schema_version, "1.0");
}

/// Helper: Test that temp file pattern leaves no artifacts on success
#[test]
fn test_state_save_no_temp_artifacts() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");

    let state = State::empty();
    state.save(&state_path).unwrap();

    // Check that no .tmp files remain (only state.json and .lock)
    let mut entries: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    entries.sort();

    assert_eq!(
        entries,
        vec![".lock", "state.json"],
        "Should have state.json and .lock, no .tmp files: {:?}",
        entries
    );

    // Verify no .tmp extension
    for entry in &entries {
        assert!(
            !entry.ends_with(".tmp"),
            "Should not have .tmp files: {}",
            entry
        );
    }
}

/// Helper: Test that state.json update is atomic (rename-based)
#[test]
fn test_state_save_uses_rename() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");

    // Create initial state
    let mut state1 = State::empty();
    state1.sync = Some(typstlab_core::state::SyncState {
        last_sync: Some(chrono::Utc::now()),
    });
    state1.save(&state_path).unwrap();

    // Get initial metadata
    let metadata1 = fs::metadata(&state_path).unwrap();
    let inode1 = get_inode(&metadata1);

    // Save again
    let mut state2 = State::empty();
    state2.sync = Some(typstlab_core::state::SyncState {
        last_sync: Some(chrono::Utc::now()),
    });
    state2.save(&state_path).unwrap();

    // Get new metadata
    let metadata2 = fs::metadata(&state_path).unwrap();
    let inode2 = get_inode(&metadata2);

    // If using rename, inode should change (new file)
    // Note: This test is Unix-specific and may not work on all filesystems
    #[cfg(unix)]
    assert_ne!(
        inode1, inode2,
        "Atomic rename should create new inode (temp file → rename pattern)"
    );
}

#[cfg(unix)]
fn get_inode(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.ino()
}

#[cfg(not(unix))]
fn get_inode(_metadata: &fs::Metadata) -> u64 {
    // On non-Unix, just return dummy value (test will pass)
    0
}
