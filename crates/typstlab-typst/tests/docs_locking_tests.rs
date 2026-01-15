//! Integration tests for docs sync locking (parallel syncs)
//!
//! These tests verify that file locking prevents corruption when multiple
//! threads/processes try to sync documentation for the same project simultaneously.
//!
//! Uses fixtures-based approach with real HTML files to avoid PathTraversal errors.

use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;
use typstlab_typst::docs::sync_docs;
use typstlab_typst::docs::test_helpers::{
    clear_mock_github_url, load_docs_archive_from_fixtures, mock_github_docs_release,
    set_mock_github_url,
};

#[test]
fn test_parallel_docs_sync_no_corruption() {
    use mockito::Server;

    // Setup: Create mock HTTP server
    let mut server = Server::new();
    let archive_bytes = load_docs_archive_from_fixtures();

    // With locking, only ONE download should occur:
    // - First thread acquires lock and downloads
    // - Other threads wait for lock
    // - When they get lock, docs already exist -> early exit
    let mock = mock_github_docs_release(&mut server, "0.12.0", &archive_bytes)
        .expect(1) // With locking, only first thread downloads
        .create();

    // Override GitHub base URL for testing
    set_mock_github_url(&server.url());

    let temp_project = TempDir::new().unwrap();
    let kb_dir = temp_project.path().join(".typstlab").join("kb");
    let target_dir = kb_dir.join("typst").join("docs");

    // Create parent directories
    fs::create_dir_all(target_dir.parent().unwrap()).unwrap();

    const NUM_THREADS: usize = 3;
    let barrier = Arc::new(Barrier::new(NUM_THREADS));

    // Spawn 3 threads that all try to sync docs simultaneously
    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|_| {
            let target_dir = target_dir.clone();
            let barrier = Arc::clone(&barrier);

            thread::spawn(move || {
                barrier.wait(); // Synchronize start

                // All threads try to sync same docs
                let result = sync_docs("0.12.0", &target_dir, false);
                result.unwrap()
            })
        })
        .collect();

    // Wait for all threads and collect results
    let file_counts: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Verify: All threads got same file count
    assert_eq!(
        file_counts.len(),
        NUM_THREADS,
        "All threads should complete"
    );
    for count in &file_counts[1..] {
        assert_eq!(
            count, &file_counts[0],
            "All threads should return same file count"
        );
    }

    // Verify: Docs directory exists and has files
    assert!(target_dir.exists(), "Docs directory should exist");
    let actual_file_count = fs::read_dir(&target_dir).unwrap().count();
    assert!(actual_file_count > 0, "Docs directory should have files");

    // Verify: Only one download occurred (mock expectation)
    mock.assert();

    // Cleanup
    clear_mock_github_url();
}

#[test]
fn test_docs_sync_idempotency_with_locking() {
    use mockito::Server;

    // Setup: Create mock HTTP server
    let mut server = Server::new();
    let archive_bytes = load_docs_archive_from_fixtures();

    // Only first sync should download
    let mock = mock_github_docs_release(&mut server, "0.12.0", &archive_bytes)
        .expect(1) // Only first sync downloads
        .create();

    set_mock_github_url(&server.url());

    let temp_project = TempDir::new().unwrap();
    let kb_dir = temp_project.path().join(".typstlab").join("kb");
    let target_dir = kb_dir.join("typst").join("docs");

    fs::create_dir_all(target_dir.parent().unwrap()).unwrap();

    // First sync (verbose for debugging)
    let count1 = sync_docs("0.12.0", &target_dir, true).unwrap();
    assert!(count1 > 0, "First sync should extract files");

    // Second sync (should be idempotent - early exit without download)
    let count2 = sync_docs("0.12.0", &target_dir, false).unwrap();

    // Verify: Same file count (no re-download/duplication)
    assert_eq!(count1, count2, "Second sync should return same count");

    // Verify: Only one download occurred
    mock.assert();

    // Cleanup
    clear_mock_github_url();
}

#[test]
fn test_concurrent_docs_sync_different_projects_no_conflict() {
    use mockito::Server;

    // Setup: Create mock HTTP server
    let mut server = Server::new();
    let archive_bytes = load_docs_archive_from_fixtures();

    // Different projects should download independently (2 downloads)
    let mock = mock_github_docs_release(&mut server, "0.12.0", &archive_bytes)
        .expect(2) // Two different projects = 2 downloads
        .create();

    set_mock_github_url(&server.url());

    // Create two different project directories
    let project1 = TempDir::new().unwrap();
    let project2 = TempDir::new().unwrap();

    let target_dir1 = project1
        .path()
        .join(".typstlab")
        .join("kb")
        .join("typst")
        .join("docs");
    let target_dir2 = project2
        .path()
        .join(".typstlab")
        .join("kb")
        .join("typst")
        .join("docs");

    fs::create_dir_all(target_dir1.parent().unwrap()).unwrap();
    fs::create_dir_all(target_dir2.parent().unwrap()).unwrap();

    const NUM_THREADS: usize = 2;
    let barrier = Arc::new(Barrier::new(NUM_THREADS));

    // Spawn threads for different projects
    let target_dir1_clone = target_dir1.clone();
    let barrier1 = Arc::clone(&barrier);
    let handle1 = thread::spawn(move || {
        barrier1.wait();
        sync_docs("0.12.0", &target_dir1_clone, false).unwrap()
    });

    let target_dir2_clone = target_dir2.clone();
    let barrier2 = Arc::clone(&barrier);
    let handle2 = thread::spawn(move || {
        barrier2.wait();
        sync_docs("0.12.0", &target_dir2_clone, false).unwrap()
    });

    // Wait for both
    let count1 = handle1.join().unwrap();
    let count2 = handle2.join().unwrap();

    // Verify: Both syncs succeeded
    assert!(count1 > 0, "Project 1 should have docs");
    assert!(count2 > 0, "Project 2 should have docs");
    assert_eq!(count1, count2, "Both should have same number of docs");

    // Verify: Both docs directories exist
    assert!(target_dir1.exists(), "Project 1 docs should exist");
    assert!(target_dir2.exists(), "Project 2 docs should exist");

    // Verify: Two downloads occurred (different projects, different locks)
    mock.assert();

    // Cleanup
    clear_mock_github_url();
}
