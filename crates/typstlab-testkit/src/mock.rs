//! Mock server infrastructure for testing
//!
//! This module provides a shared mockito server for parallel test execution.
//! Using a single shared server eliminates environment variable conflicts
//! and enables true parallel testing.

use lazy_static::lazy_static;
use mockito::{Server, ServerGuard};
use std::sync::Mutex;

lazy_static! {
    /// Global shared mockito server for all tests
    ///
    /// This server is initialized once and shared across all test threads.
    /// Benefits:
    /// - Eliminates environment variable conflicts
    /// - Enables true parallel test execution
    /// - Provides consistent base URL for all tests
    pub static ref SHARED_MOCK_SERVER: Mutex<ServerGuard> = Mutex::new(Server::new());
}

/// Get reference to shared mock server
///
/// This function provides access to the global mockito server.
/// The server is initialized lazily on first access.
///
/// # Thread Safety
///
/// The server is protected by a Mutex to ensure thread-safe access
/// when creating/removing mocks.
///
/// # Best Practices for Avoiding Mock Collisions
///
/// When writing parallel tests that use the shared server:
///
/// 1. **Use unique paths per test**: Different tests should mock different URLs
///    to avoid conflicts (e.g., `/v0.12.0/...`, `/v0.13.0/...`, `/v0.14.0/...`)
/// 2. **Mock cleanup is automatic**: Mocks are removed when the Mock object drops
/// 3. **Lock scope matters**: Acquire the server lock only during mock setup/teardown,
///    not during the entire test execution
///
/// # Examples
///
/// ```no_run
/// use typstlab_testkit::get_shared_mock_server;
///
/// // Example test function (not executed in doctest)
/// fn test_with_shared_server() {
///     // Acquire lock only for mock setup
///     let mock = {
///         let mut server = get_shared_mock_server();
///         server.mock("GET", "/unique-path-v1/resource")  // Use unique path
///             .with_status(200)
///             .create()
///     }; // Lock released here
///
///     // Test logic runs with server unlocked (parallel execution!)
///     // ...
///
///     // Mock automatically cleaned up when dropped
/// }
/// ```
pub fn get_shared_mock_server() -> std::sync::MutexGuard<'static, ServerGuard> {
    SHARED_MOCK_SERVER.lock().unwrap_or_else(|poisoned| {
        // Recover from poisoned mutex
        // Safe because:
        // - Mockito server remains functional after panic
        // - We're just serializing access
        // - Test isolation still maintained via unique mock paths
        poisoned.into_inner()
    })
}

/// Initialize shared mock GitHub base URL
///
/// Sets the GITHUB_BASE_URL environment variable to point to the shared
/// mock server. This is **idempotent** - calling multiple times is safe and
/// efficient (no-op if already set).
///
/// # Important Notes
///
/// - **Persistent**: GITHUB_BASE_URL remains set for the entire process lifetime
/// - **Shared**: All tests use the same mock server URL (intentional design)
/// - **Mock Isolation**: Individual mocks are automatically cleaned up when dropped
/// - **No Restoration**: The original GITHUB_BASE_URL value is not restored
///   (by design, as all tests share the same mock server)
///
/// # Thread Safety
///
/// This function is thread-safe and can be called concurrently from multiple tests.
///
/// # Examples
///
/// ```no_run
/// use typstlab_testkit::init_shared_mock_github_url;
///
/// // Example test function (not executed in doctest)
/// fn test_github_interaction() {
///     init_shared_mock_github_url();  // Safe to call in every test
///     // Test code that uses github_base_url()
/// }
/// ```
pub fn init_shared_mock_github_url() {
    // Check if already initialized (idempotent)
    if std::env::var("GITHUB_BASE_URL").is_ok() {
        return; // Already set, no-op
    }

    let server = get_shared_mock_server();
    let url = server.url();
    unsafe {
        std::env::set_var("GITHUB_BASE_URL", url);
    }
}

// Note: Poison recovery tests moved to integration tests
// See crates/typstlab-testkit/tests/poison_recovery.rs
