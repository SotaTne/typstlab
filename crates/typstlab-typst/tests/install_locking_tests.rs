//! Integration tests for install locking (parallel installs of same version)
//!
//! These tests verify that file locking prevents corruption when multiple
//! threads try to install the same Typst version simultaneously.

use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;
use typstlab_typst::install::download::{DownloadOptions, download_and_install};
use typstlab_typst::install::platform::binary_name;
use typstlab_typst::install::release::Asset;
use url::Url;

/// Creates a mock Asset for testing
fn mock_asset(name: &str, url: &str, size: u64) -> Asset {
    Asset {
        name: name.to_string(),
        browser_download_url: Url::parse(url).unwrap(),
        size,
    }
}

/// Helper: Create a fake .tar.xz archive with a binary inside
///
/// This is adapted from download.rs tests to create a valid test archive.
fn create_fake_tar_xz_with_binary(binary_name_str: &str, nested: bool) -> TempDir {
    let temp = TempDir::new().unwrap();
    let binary_content = "#!/bin/sh\necho 'typst 0.12.0'".to_string();

    // Determine platform-specific directory name
    let nested_dir_name = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "x86_64") {
            "typst-x86_64-apple-darwin"
        } else {
            "typst-aarch64-apple-darwin"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "x86_64") {
            "typst-x86_64-unknown-linux-musl"
        } else {
            "typst-aarch64-unknown-linux-musl"
        }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            "typst-x86_64-pc-windows-msvc"
        } else {
            "typst-aarch64-pc-windows-msvc"
        }
    } else {
        "typst-unknown-platform"
    };

    // Create binary in appropriate location
    let binary_path = if nested {
        let nested_dir = temp.path().join(nested_dir_name);
        fs::create_dir_all(&nested_dir).unwrap();
        nested_dir.join(binary_name_str)
    } else {
        temp.path().join(binary_name_str)
    };

    fs::write(&binary_path, binary_content).unwrap();

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms).unwrap();
    }

    // Create .tar.xz archive with proper XZ compression
    let archive_path = temp.path().join("archive.tar.xz");
    let tar_xz = std::fs::File::create(&archive_path).unwrap();
    let enc = xz2::write::XzEncoder::new(tar_xz, 6);
    let mut tar = tar::Builder::new(enc);

    let binary_rel_path = if nested {
        format!("{}/{}", nested_dir_name, binary_name_str)
    } else {
        binary_name_str.to_string()
    };

    tar.append_path_with_name(&binary_path, &binary_rel_path)
        .unwrap();
    tar.finish().unwrap();

    temp
}

#[test]
fn test_parallel_install_same_version_no_corruption() {
    // Verify: Multiple threads installing same version → only one downloads
    // Without locking, this test may expose race conditions where:
    // - Both threads try to create version directory
    // - Both threads try to write same binary
    // - Binary gets corrupted or truncated

    use mockito::Server;

    let mut server = Server::new();

    // Create a real tar.xz archive with a binary for testing
    let archive_temp = create_fake_tar_xz_with_binary(binary_name(), true);
    let archive_path = archive_temp.path().join("archive.tar.xz");
    let archive_bytes = fs::read(&archive_path).unwrap();
    let archive_size = archive_bytes.len() as u64;

    // Set up mock response
    // With locking, only ONE download should occur:
    // - First thread acquires lock and downloads
    // - Other threads wait for lock
    // - When they get lock, binary already exists -> early exit
    let mock = server
        .mock("GET", "/typst.tar.xz")
        .with_status(200)
        .with_header("content-type", "application/x-xz")
        .with_body(&archive_bytes)
        .expect(1) // With locking, only first thread downloads
        .create();

    let cache_dir = TempDir::new().unwrap();
    let cache_dir_path = cache_dir.path().to_path_buf();

    const NUM_THREADS: usize = 3;
    let barrier = Arc::new(Barrier::new(NUM_THREADS));

    // Spawn 3 threads that all try to install v0.12.0 simultaneously
    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|_| {
            let cache_dir = cache_dir_path.clone();
            let barrier = Arc::clone(&barrier);
            let mock_url = format!("{}/typst.tar.xz", server.url());
            let archive_size_clone = archive_size;

            thread::spawn(move || {
                barrier.wait(); // Synchronize start

                // All try to install same version
                let asset = mock_asset(
                    "typst-x86_64-apple-darwin.tar.xz",
                    &mock_url,
                    archive_size_clone,
                );

                let options = DownloadOptions {
                    cache_dir: cache_dir.clone(),
                    version: "0.12.0".to_string(),
                    progress: None,
                };

                download_and_install(&asset, options).unwrap()
            })
        })
        .collect();

    // Wait for all
    let paths: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    mock.assert();

    // Verify: All got same binary path
    assert_eq!(paths.len(), NUM_THREADS);
    for path in &paths[1..] {
        assert_eq!(
            path, &paths[0],
            "All threads should return same binary path"
        );
    }

    // Verify: Binary is valid (not corrupted)
    let binary_path = &paths[0];
    assert!(binary_path.exists(), "Binary should exist");
    let metadata = fs::metadata(binary_path).unwrap();
    assert!(
        metadata.len() > 0,
        "Binary should have content (not truncated)"
    );

    // Verify: Binary content is correct
    let content = fs::read_to_string(binary_path).unwrap();
    assert!(
        content.contains("typst 0.12.0"),
        "Binary should contain expected version string"
    );
}

#[test]
fn test_install_idempotency_with_locking() {
    // Verify: Installing already-installed version exits early
    // This tests the optimization where install checks if binary exists
    // before acquiring lock and downloading.

    use mockito::Server;

    let mut server = Server::new();

    // Create a real tar.xz archive with a binary for testing
    let archive_temp = create_fake_tar_xz_with_binary(binary_name(), true);
    let archive_path = archive_temp.path().join("archive.tar.xz");
    let archive_bytes = fs::read(&archive_path).unwrap();
    let archive_size = archive_bytes.len() as u64;

    // Set up mock response - expect only 1 request (not 2)
    let mock = server
        .mock("GET", "/typst.tar.xz")
        .with_status(200)
        .with_header("content-type", "application/x-xz")
        .with_body(&archive_bytes)
        .expect(1) // Only first install should download
        .create();

    let cache_dir = TempDir::new().unwrap();
    let mock_url = format!("{}/typst.tar.xz", server.url());

    let asset = mock_asset("typst-x86_64-apple-darwin.tar.xz", &mock_url, archive_size);

    let options = DownloadOptions {
        cache_dir: cache_dir.path().to_path_buf(),
        version: "0.12.0".to_string(),
        progress: None,
    };

    // First install
    let path1 = download_and_install(&asset, options.clone()).unwrap();
    let metadata1 = fs::metadata(&path1).unwrap();

    // Second install (should exit early without re-download)
    let path2 = download_and_install(&asset, options).unwrap();
    let metadata2 = fs::metadata(&path2).unwrap();

    mock.assert();

    // Verify: Same path, same file (no re-download)
    assert_eq!(path1, path2, "Should return same path");
    assert_eq!(
        metadata1.len(),
        metadata2.len(),
        "File should not change on second install"
    );

    // Verify: Binary is still valid
    let content = fs::read_to_string(&path2).unwrap();
    assert!(
        content.contains("typst 0.12.0"),
        "Binary should still be valid after second install"
    );
}

#[test]
fn test_concurrent_installs_different_versions_no_conflict() {
    // Verify: Installing different versions in parallel should work
    // (locks are per-version, not global)

    use mockito::Server;

    let mut server = Server::new();

    // Create archives for two different versions
    let archive1 = create_fake_tar_xz_with_binary(binary_name(), true);
    let archive1_path = archive1.path().join("archive.tar.xz");
    let archive1_bytes = fs::read(&archive1_path).unwrap();
    let archive1_size = archive1_bytes.len() as u64;

    let archive2 = create_fake_tar_xz_with_binary(binary_name(), true);
    let archive2_path = archive2.path().join("archive.tar.xz");
    let archive2_bytes = fs::read(&archive2_path).unwrap();
    let archive2_size = archive2_bytes.len() as u64;

    // Set up mock responses for different versions
    let mock1 = server
        .mock("GET", "/typst-0.12.0.tar.xz")
        .with_status(200)
        .with_body(&archive1_bytes)
        .create();

    let mock2 = server
        .mock("GET", "/typst-0.11.0.tar.xz")
        .with_status(200)
        .with_body(&archive2_bytes)
        .create();

    let cache_dir = TempDir::new().unwrap();
    let cache_dir_path = cache_dir.path().to_path_buf();

    let barrier = Arc::new(Barrier::new(2));

    // Spawn 2 threads installing different versions
    let handle1 = {
        let cache_dir = cache_dir_path.clone();
        let barrier = Arc::clone(&barrier);
        let url = format!("{}/typst-0.12.0.tar.xz", server.url());
        let size = archive1_size;

        thread::spawn(move || {
            barrier.wait();

            let asset = mock_asset("typst-x86_64-apple-darwin.tar.xz", &url, size);
            let options = DownloadOptions {
                cache_dir,
                version: "0.12.0".to_string(),
                progress: None,
            };

            download_and_install(&asset, options).unwrap()
        })
    };

    let handle2 = {
        let cache_dir = cache_dir_path.clone();
        let barrier = Arc::clone(&barrier);
        let url = format!("{}/typst-0.11.0.tar.xz", server.url());
        let size = archive2_size;

        thread::spawn(move || {
            barrier.wait();

            let asset = mock_asset("typst-x86_64-apple-darwin.tar.xz", &url, size);
            let options = DownloadOptions {
                cache_dir,
                version: "0.11.0".to_string(),
                progress: None,
            };

            download_and_install(&asset, options).unwrap()
        })
    };

    let path1 = handle1.join().unwrap();
    let path2 = handle2.join().unwrap();

    mock1.assert();
    mock2.assert();

    // Verify: Different version directories
    assert!(
        path1.to_string_lossy().contains("0.12.0"),
        "Should install to 0.12.0 directory"
    );
    assert!(
        path2.to_string_lossy().contains("0.11.0"),
        "Should install to 0.11.0 directory"
    );

    // Verify: Both binaries exist and are valid
    assert!(path1.exists(), "Version 0.12.0 binary should exist");
    assert!(path2.exists(), "Version 0.11.0 binary should exist");

    let content1 = fs::read_to_string(&path1).unwrap();
    let content2 = fs::read_to_string(&path2).unwrap();

    assert!(content1.contains("typst"), "Binary 1 should be valid");
    assert!(content2.contains("typst"), "Binary 2 should be valid");
}
