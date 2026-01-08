# Testing Guidelines for typstlab

This document outlines best practices for writing tests in the typstlab project. Following these guidelines ensures test isolation, reliability, and maintainability.

## Table of Contents

1. [Filesystem Operations in Tests](#filesystem-operations-in-tests)
2. [Test Isolation Principles](#test-isolation-principles)
3. [Common Patterns](#common-patterns)
4. [Platform-Specific Testing](#platform-specific-testing)
5. [What Not to Do](#what-not-to-do)

---

## Filesystem Operations in Tests

### ✅ DO: Use `tempfile::TempDir`

**Always** use the `tempfile` crate for temporary filesystem operations in tests. `TempDir` provides automatic cleanup when dropped, ensuring no state leakage between tests.

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_with_temp_directory() {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();

    // Use temp_dir.path() to get the path
    let file_path = temp_dir.path().join("test_file.txt");
    fs::write(&file_path, "test data").unwrap();

    // Perform your test assertions
    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "test data");

    // Cleanup happens automatically when temp_dir is dropped
}
```

**Why this is safe:**
- Each test gets its own isolated directory
- No conflicts with parallel test execution
- Automatic cleanup even if the test panics
- No manual cleanup code needed

### ❌ DON'T: Use `env::temp_dir()` directly

**Never** use `std::env::temp_dir()` directly in test code. This leads to several problems:

```rust
// ❌ WRONG - DO NOT DO THIS
use std::env;
use std::fs;

#[test]
fn test_bad_pattern() {
    let temp_dir = env::temp_dir();  // ❌ Shared system directory
    let file = temp_dir.join("test_file.txt");
    fs::write(&file, "data").unwrap();

    // Assertions...

    // Manual cleanup - may not execute if test panics
    let _ = fs::remove_file(&file);  // ❌ Unreliable cleanup
}
```

**Problems with this approach:**
- All tests share the same system temp directory
- File name collisions between parallel tests
- Manual cleanup may not execute on panic
- Leaves garbage files in system directories
- Tests may fail due to leftover state from previous runs

---

## Test Isolation Principles

Follow these principles to ensure tests are independent and reliable:

### 1. Each Test Must Be Independent

Tests should not depend on:
- Execution order
- Other tests running before or after
- Shared global state
- Leftover files from previous test runs

### 2. Use TempDir for All Filesystem Operations

Any test that creates files, directories, or manipulates the filesystem **must** use `TempDir`.

### 3. Never Rely on Shared State

Avoid:
- Writing to fixed paths (e.g., `/tmp/my-test-file`)
- Using global variables that are mutated
- Depending on environment variables set by other tests

### 4. No Manual Cleanup Code

Rely on Rust's RAII (Resource Acquisition Is Initialization) pattern:
- `TempDir` automatically cleans up on drop
- Avoid explicit `fs::remove_file()` or `fs::remove_dir()` in tests
- If cleanup must be manual, use a `Drop` guard

---

## Common Patterns

### Pattern 1: Creating Temporary Files

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_read_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Write test config
    fs::write(&config_path, r#"
        [project]
        name = "test-project"
    "#).unwrap();

    // Test your code that reads the config
    let config = load_config(&config_path).unwrap();
    assert_eq!(config.project.name, "test-project");
}
```

### Pattern 2: Creating Directory Structures

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_directory_traversal() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested directory structure
    let project_dir = temp_dir.path().join("my-project");
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create files
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(project_dir.join("Cargo.toml"), "[package]").unwrap();

    // Test your directory traversal logic
    let files = list_project_files(&project_dir).unwrap();
    assert_eq!(files.len(), 2);
}
```

### Pattern 3: Creating Fake Executable Binaries

For testing code that executes external binaries:

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_execute_binary() {
    let temp_dir = TempDir::new().unwrap();

    #[cfg(unix)]
    let binary_path = {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_dir.path().join("fake-tool");

        // Create shell script
        fs::write(&path, "#!/bin/sh\necho 'output from fake tool'").unwrap();

        // Make executable
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();

        path
    };

    #[cfg(windows)]
    let binary_path = {
        let path = temp_dir.path().join("fake-tool.bat");
        fs::write(&path, "@echo output from fake tool").unwrap();
        path
    };

    // Test code that executes the binary
    let result = execute_tool(&binary_path).unwrap();
    assert!(result.contains("output from fake tool"));
}
```

### Pattern 4: Using Test Helpers

Create reusable test helper functions in a `test_helpers` module:

```rust
#[cfg(test)]
mod test_helpers {
    use tempfile::TempDir;
    use std::path::PathBuf;
    use std::fs;

    /// Create a fake binary that outputs a specific version
    pub fn create_fake_versioned_binary(
        temp_dir: &TempDir,
        name: &str,
        version: &str
    ) -> PathBuf {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = temp_dir.path().join(name);
            let script = format!("#!/bin/sh\necho '{} {}'", name, version);
            fs::write(&path, script).unwrap();
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
            path
        }

        #[cfg(windows)]
        {
            let path = temp_dir.path().join(format!("{}.bat", name));
            let script = format!("@echo {} {}", name, version);
            fs::write(&path, script).unwrap();
            path
        }
    }
}

#[test]
fn test_using_helper() {
    use test_helpers::create_fake_versioned_binary;

    let temp_dir = TempDir::new().unwrap();
    let binary = create_fake_versioned_binary(&temp_dir, "my-tool", "1.0.0");

    // Test code...
}
```

---

## Platform-Specific Testing

When writing tests that behave differently on different platforms:

### Use `cfg` Attributes

```rust
#[test]
fn test_platform_specific_behavior() {
    let temp_dir = TempDir::new().unwrap();

    #[cfg(unix)]
    {
        // Unix-specific test logic
        use std::os::unix::fs::PermissionsExt;
        let path = temp_dir.path().join("executable");
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        // ...
    }

    #[cfg(windows)]
    {
        // Windows-specific test logic
        let path = temp_dir.path().join("executable.exe");
        // ...
    }
}
```

### Platform-Specific Test Functions

```rust
#[test]
#[cfg(target_os = "macos")]
fn test_macos_specific() {
    // This test only runs on macOS
}

#[test]
#[cfg(target_os = "linux")]
fn test_linux_specific() {
    // This test only runs on Linux
}

#[test]
#[cfg(target_os = "windows")]
fn test_windows_specific() {
    // This test only runs on Windows
}
```

---

## What Not to Do

### ❌ Hardcoded Paths

```rust
// ❌ WRONG
#[test]
fn test_bad() {
    let file = PathBuf::from("/tmp/test-file");
    fs::write(&file, "data").unwrap();
    // ...
}
```

### ❌ Manual Cleanup in Tests

```rust
// ❌ WRONG - cleanup may not run if test panics
#[test]
fn test_bad() {
    let temp_dir = env::temp_dir();
    let file = temp_dir.join("test.txt");
    fs::write(&file, "data").unwrap();

    // test logic...

    fs::remove_file(&file).unwrap();  // May not execute!
}
```

### ❌ Shared Test State

```rust
// ❌ WRONG - tests will interfere with each other
static TEST_FILE: &str = "/tmp/shared-test-file.txt";

#[test]
fn test_a() {
    fs::write(TEST_FILE, "data from test A").unwrap();
    // ...
}

#[test]
fn test_b() {
    fs::write(TEST_FILE, "data from test B").unwrap();
    // ...
}
```

### ❌ Tests That Depend on Execution Order

```rust
// ❌ WRONG - test_b depends on test_a running first
#[test]
fn test_a_creates_file() {
    fs::write("/tmp/data.txt", "data").unwrap();
}

#[test]
fn test_b_reads_file() {
    // This will fail if test_a hasn't run!
    let data = fs::read_to_string("/tmp/data.txt").unwrap();
    assert_eq!(data, "data");
}
```

---

## Integration with typstlab-testkit (Future)

In the future, common test utilities may be consolidated in the `typstlab-testkit` crate. When available, prefer using these shared utilities:

```rust
// Future usage (not yet implemented)
use typstlab_testkit::{temp_dir, create_fake_binary};

#[test]
fn test_example() {
    let dir = temp_dir();
    let binary = create_fake_binary(dir.path(), "my-tool", "1.0.0");
    // ...
}
```

---

## References

- [`tempfile` crate documentation](https://docs.rs/tempfile/)
- [Rust testing best practices](https://doc.rust-lang.org/book/ch11-00-testing.html)
- typstlab project structure: See `DESIGN.md`

---

## Questions or Issues?

If you have questions about these testing guidelines or encounter situations not covered here, please:

1. Check existing test code in `crates/typstlab-typst/tests/` for examples
2. Review the `test_helpers` module in `crates/typstlab-typst/src/resolve.rs`
3. Open a discussion in the project repository

**Remember:** When in doubt, use `TempDir`. It's the safe, reliable choice for filesystem operations in tests.
