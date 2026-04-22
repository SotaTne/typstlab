//! Integration tests for docs sync locking (parallel syncs)
//!
//! These tests verify that file locking prevents corruption when multiple
//! threads/processes try to sync documentation for the same project simultaneously.
//!
//! Uses fixtures-based approach with real HTML files to avoid PathTraversal errors.
//! Uses shared mockito server from testkit to enable true parallel execution.

use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;
use typstlab_testkit::{get_shared_mock_server, init_shared_mock_github_url, with_isolated_typst_env};
use typstlab_typst::docs::test_helpers::{load_docs_json_from_fixtures, mock_github_docs_json};

#[test]
fn test_parallel_docs_sync_no_corruption() {
    // Note: We use threads here, but since OS file locks (flock) are often 
    // process-scoped, multiple threads in the same process might not be 
    // excluded by flock alone. 
    // To make this library OSS-ready and thread-safe, we should use 
    // in-process Mutexes in typstlab-core::lock.
    
    with_isolated_typst_env(None, |cache| {
        let cache_root = cache.to_path_buf();
        init_shared_mock_github_url();
        let json_bytes = load_docs_json_from_fixtures();

        let mock = {
            let mut server = get_shared_mock_server();
            mock_github_docs_json(&mut server, "0.12.0", &json_bytes)
                .expect(1)
                .create()
        };

        let temp_project = TempDir::new().unwrap();
        let kb_dir = temp_project.path().join(".typstlab").join("kb");
        let target_dir = kb_dir.join("typst").join("docs");
        fs::create_dir_all(target_dir.parent().unwrap()).unwrap();

        const NUM_THREADS: usize = 3;
        let barrier = Arc::new(Barrier::new(NUM_THREADS));

        let handles: Vec<_> = (0..NUM_THREADS)
            .map(|_| {
                let target_dir = target_dir.clone();
                let barrier = Arc::clone(&barrier);
                let cache_root = cache_root.clone();

                thread::spawn(move || {
                    barrier.wait();
                    // sync_docs should be thread-safe now
                    let env = typstlab_core::context::Environment {
                        cache_root,
                        cwd: std::env::current_dir().unwrap(),
                    };
                    let result = typstlab_typst::docs::sync_docs_with_env(&env, "0.12.0", &target_dir, false);
                    result.unwrap()
                })
            })
            .collect();

        let file_counts: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(file_counts.len(), NUM_THREADS);
        for count in &file_counts[1..] {
            assert_eq!(count, &file_counts[0]);
        }

        assert!(target_dir.exists());
        mock.assert();
    });
}

#[test]
fn test_docs_sync_idempotency_with_locking() {
    with_isolated_typst_env(None, |_cache| {
        // Initialize shared mock GitHub URL
        init_shared_mock_github_url();

        let json_bytes = load_docs_json_from_fixtures();

        // Use shared server - lock only during mock setup
        let mock = {
            let mut server = get_shared_mock_server();
            mock_github_docs_json(&mut server, "0.12.0", &json_bytes)
                .expect(1) // Only first sync downloads
                .create()
        };

        let temp_project = TempDir::new().unwrap();
        let kb_dir = temp_project.path().join(".typstlab").join("kb");
        let target_dir = kb_dir.join("typst").join("docs");

        fs::create_dir_all(target_dir.parent().unwrap()).unwrap();

        let env = typstlab_core::context::Environment {
            cache_root: _cache.to_path_buf(),
            cwd: std::env::current_dir().unwrap(),
        };

        // First sync (verbose for debugging)
        let count1 = typstlab_typst::docs::sync_docs_with_env(&env, "0.12.0", &target_dir, true).unwrap();
        assert!(count1 > 0, "First sync should extract files");

        // Second sync (should be idempotent - early exit without download)
        let count2 = typstlab_typst::docs::sync_docs_with_env(&env, "0.12.0", &target_dir, false).unwrap();

        // Verify: Same file count (no re-download/duplication)
        assert_eq!(count1, count2, "Second sync should return same count");

        // Verify: Only one download occurred
        mock.assert();
    });
}

#[test]
fn test_concurrent_docs_sync_different_projects_no_conflict() {
    with_isolated_typst_env(None, |_cache| {
        // Initialize shared mock GitHub URL
        init_shared_mock_github_url();

        let json_bytes = load_docs_json_from_fixtures();

        // Use shared server - lock only during mock setup
        let mock = {
            let mut server = get_shared_mock_server();
            mock_github_docs_json(&mut server, "0.12.0", &json_bytes)
                .expect(2) // Two different projects = 2 downloads
                .create()
        };

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
        let cache_root1 = _cache.join("p1"); // Unique cache for project 1
        let handle1 = thread::spawn(move || {
            barrier1.wait();
            let env = typstlab_core::context::Environment {
                cache_root: cache_root1,
                cwd: std::env::current_dir().unwrap(),
            };
            typstlab_typst::docs::sync_docs_with_env(&env, "0.12.0", &target_dir1_clone, false).unwrap()
        });

        let target_dir2_clone = target_dir2.clone();
        let barrier2 = Arc::clone(&barrier);
        let cache_root2 = _cache.join("p2"); // Unique cache for project 2
        let handle2 = thread::spawn(move || {
            barrier2.wait();
            let env = typstlab_core::context::Environment {
                cache_root: cache_root2,
                cwd: std::env::current_dir().unwrap(),
            };
            typstlab_typst::docs::sync_docs_with_env(&env, "0.12.0", &target_dir2_clone, false).unwrap()
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
    });
}

