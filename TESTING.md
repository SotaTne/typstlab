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

## End-to-End (E2E) Testing

End-to-end tests simulate real user workflows by testing the full system integration. These tests are more complex than unit tests and require special attention to isolation and safety.

### ⚠️ Dangers of E2E Testing

E2E tests can be dangerous if not written carefully:

1. **Modifying actual project files** - Tests might accidentally modify the real project being developed
2. **Using global state** - Tests might pollute global configurations, environment variables, or home directories
3. **Non-deterministic behavior** - Tests might depend on the current working directory or system state
4. **Data loss** - Poorly written tests might delete or overwrite important files

### ✅ Safe E2E Testing Patterns

#### Pattern 1: Isolated Test Projects with TempDir

**Always** create isolated test projects in temporary directories:

```rust
use tempfile::TempDir;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_e2e_workflow() {
    // Create isolated test project
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Set up project structure
    fs::create_dir_all(project_root.join("papers/paper1/rules")).unwrap();
    fs::write(
        project_root.join("typstlab.toml"),
        r#"
[project]
name = "test-project"
version = "0.1.0"
        "#
    ).unwrap();
    fs::write(
        project_root.join("papers/paper1/main.typ"),
        "= Test Paper"
    ).unwrap();

    // Run your E2E test against this isolated project
    let result = run_typstlab_command(&project_root, &["build", "paper1"]);
    assert!(result.is_ok());

    // Verify expected outputs
    assert!(project_root.join("build/paper1.pdf").exists());

    // Cleanup happens automatically when temp_dir is dropped
}
```

**Why this is safe:**

- Each test gets its own isolated project directory
- No risk of modifying the actual project being developed
- No interference with parallel test execution
- Automatic cleanup even if the test panics

#### Pattern 2: TestProject Helper Struct

Create a reusable helper for setting up test projects:

```rust
#[cfg(test)]
mod test_helpers {
    use tempfile::TempDir;
    use std::path::{Path, PathBuf};
    use std::fs;

    /// A self-contained test project with automatic cleanup
    pub struct TestProject {
        _temp_dir: TempDir,
        root: PathBuf,
    }

    impl TestProject {
        /// Create a new test project with typical structure
        pub fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let root = temp_dir.path().to_path_buf();

            // Create standard project structure
            fs::create_dir_all(root.join("papers")).unwrap();
            fs::create_dir_all(root.join("rules")).unwrap();
            fs::write(
                root.join("typstlab.toml"),
                r#"
[project]
name = "test-project"
version = "0.1.0"
                "#
            ).unwrap();

            Self {
                _temp_dir: temp_dir,
                root,
            }
        }

        /// Get the root path of the test project
        pub fn root(&self) -> &Path {
            &self.root
        }

        /// Add a paper to the test project
        pub fn add_paper(&self, paper_id: &str, content: &str) -> &Self {
            let paper_dir = self.root.join("papers").join(paper_id);
            fs::create_dir_all(&paper_dir).unwrap();
            fs::write(paper_dir.join("main.typ"), content).unwrap();
            fs::write(
                paper_dir.join("paper.toml"),
                format!(r#"id = "{}""#, paper_id)
            ).unwrap();
            self
        }

        /// Add a rule file to the test project
        pub fn add_rule(&self, paper_id: &str, filename: &str, content: &str) -> &Self {
            let rules_dir = self.root.join("papers").join(paper_id).join("rules");
            fs::create_dir_all(&rules_dir).unwrap();
            fs::write(rules_dir.join(filename), content).unwrap();
            self
        }
    }
}

// Usage example:
#[test]
fn test_e2e_with_helper() {
    use test_helpers::TestProject;

    let project = TestProject::new()
        .add_paper("paper1", "= Test Paper")
        .add_rule("paper1", "guide.md", "# Writing Guidelines");

    // Run E2E tests against the test project
    let result = run_typstlab_command(project.root(), &["build", "paper1"]);
    assert!(result.is_ok());

    // Cleanup happens automatically when project is dropped
}
```

**Benefits:**

- Fluent API for setting up complex test projects
- Encapsulates common setup patterns
- Automatic cleanup via `Drop`
- Clear separation between test fixture and test logic

#### Pattern 3: Testing MCP Tools with Isolated Projects

When testing MCP tools that access project files:

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_mcp_rules_get() {
    // Create isolated test project (NOT env::current_dir()!)
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Set up test data
    fs::create_dir_all(project_root.join("rules")).unwrap();
    fs::write(
        project_root.join("rules/test.md"),
        "# Test Rule\n\nContent here."
    ).unwrap();

    // Test MCP tool with explicit project_root
    let result = rules_get(project_root, "rules/test.md");
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.content.contains("Test Rule"));
}
```

**Key points:**

- Pass `project_root` explicitly to MCP tools
- Never rely on `env::current_dir()` or implicit working directory
- Create all test data in the `TempDir`

### ❌ Dangerous E2E Patterns to Avoid

#### ❌ Using env::current_dir() in Tests

```rust
// ❌ WRONG - DO NOT DO THIS
#[test]
fn test_dangerous() {
    let project_root = env::current_dir().unwrap();  // ❌ Uses actual project!

    // This could modify your actual project files!
    fs::write(project_root.join("test.txt"), "data").unwrap();

    // Run command in actual project directory (DANGEROUS!)
    let result = run_command(&project_root, &["build"]);
}
```

**Why this is dangerous:**

- Modifies files in your actual development project
- Could delete or overwrite important files
- Non-deterministic (depends on where tests are run from)
- Can't run in parallel safely

#### ❌ Testing Against Home Directory or Global Configs

```rust
// ❌ WRONG - DO NOT DO THIS
#[test]
fn test_dangerous_home_dir() {
    let home_dir = dirs::home_dir().unwrap();  // ❌ Uses actual home directory!

    // This modifies your real config!
    fs::write(
        home_dir.join(".typstlab/config.toml"),
        "test config"
    ).unwrap();
}
```

**Why this is dangerous:**

- Modifies your real user configuration
- Can break your actual typstlab installation
- State persists between test runs
- Interferes with other tests and real usage

#### ❌ Tests That Modify Fixed Locations

```rust
// ❌ WRONG - DO NOT DO THIS
#[test]
fn test_dangerous_fixed_path() {
    let test_project = PathBuf::from("/tmp/test-project");  // ❌ Fixed path!

    // Creates directory at fixed location
    fs::create_dir_all(&test_project).unwrap();

    // Multiple tests will conflict!
    fs::write(test_project.join("data.txt"), "test").unwrap();
}
```

**Why this is dangerous:**

- Tests running in parallel will conflict
- Manual cleanup may not execute on panic
- Leaves garbage in system directories
- Non-deterministic failures

### ✅ E2E Testing Checklist

Before writing an E2E test, ensure:

- [ ] Using `TempDir::new()` for test project isolation
- [ ] **Never** using `env::current_dir()` in test code
- [ ] **Never** modifying home directory or global configs
- [ ] **Never** using fixed paths like `/tmp/test-project`
- [ ] Passing explicit `project_root` parameter to functions under test
- [ ] No manual cleanup code (rely on `Drop`)
- [ ] Test works when run in parallel with other tests
- [ ] Test passes when run multiple times consecutively

### Example: Complete E2E Test

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_complete_e2e_workflow() {
    // 1. Create isolated test project
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // 2. Set up project structure
    fs::create_dir_all(project_root.join("papers/paper1/rules")).unwrap();
    fs::write(
        project_root.join("typstlab.toml"),
        r#"
[project]
name = "test-project"
        "#
    ).unwrap();
    fs::write(
        project_root.join("papers/paper1/main.typ"),
        "= Test Paper\n\n#lorem(100)"
    ).unwrap();
    fs::write(
        project_root.join("papers/paper1/rules/style.md"),
        "# Style Guide\n\nUse proper citations."
    ).unwrap();

    // 3. Test MCP tool against isolated project
    let result = rules_get(project_root, "papers/paper1/rules/style.md");
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.content.contains("Style Guide"));
    assert!(output.content.contains("citations"));

    // 4. Test build workflow
    let build_result = build_paper(project_root, "paper1");
    assert!(build_result.is_ok());

    // 5. Verify outputs
    let pdf_path = project_root.join("build/paper1.pdf");
    assert!(pdf_path.exists(), "PDF should be generated");

    // 6. No cleanup needed - TempDir handles it automatically
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
