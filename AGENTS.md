# AGENTS.md - AI-Assisted Development Guidelines

**Version**: 0.1.0
**Target**: Claude Code and other AI development assistants
**Last Updated**: 2026-01-09

---

## Purpose

This document defines development guidelines for AI-assisted development of typstlab. These rules ensure code quality, consistency, and maintainability when working with Claude Code or similar AI assistants.

---

## Core Principles

### 1. Test-Driven Development (TDD) is Mandatory

**Always write tests first, then implementation.**

#### Why TDD?

- Tests are the specification
- Prevents regressions
- Documents behavior explicitly
- Enables confident refactoring

#### TDD Workflow

1. **Red Phase**: Write failing tests that define the desired behavior
2. **Green Phase**: Write minimal implementation to make tests pass
3. **Refactor Phase**: Improve code quality while keeping tests green

#### Example

```bash
# Step 1: Write test
# Step 2: Verify test fails
cargo test test_new_feature  # Should fail

# Step 3: Implement feature
# Step 4: Verify test passes
cargo test test_new_feature  # Should pass

# Step 5: Refactor if needed
# Step 6: Verify no regressions
cargo test --workspace
```

#### Exceptions

TDD may be skipped only for:
- Pure refactoring (existing tests cover behavior)
- Documentation-only changes
- Trivial typo fixes

**For all new features and bug fixes, TDD is non-negotiable.**

---

### 2. Work in Atomic Commits

**Each commit should be a complete, logical unit of work.**

#### Commit Guidelines

- **One concern per commit**: Don't mix refactoring with feature additions
- **Complete**: Each commit should leave the codebase in a working state
- **Well-documented**: Use descriptive commit messages following Conventional Commits

#### Commit Message Format

```
<type>(<scope>): <short summary>

<detailed description>

<context and rationale>

<testing notes>

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Types**: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

#### Auto-commit Policy

**Decision left to the developer:**
- AI assistant may suggest committing after completing a logical unit
- Developer chooses whether to commit immediately or review first
- If uncertain, ask the developer before committing

#### Pre-Commit Verification (Mandatory)

**Before creating any commit, ALWAYS verify all checks pass:**

```bash
# Run these commands in order before every commit
cargo fmt --all                                      # Format code
cargo clippy --workspace --all-targets -- -D warnings  # Check for warnings
cargo test --workspace                                # Run all tests
cargo build --workspace                               # Verify build succeeds
```

**Verification Checklist:**

- ✅ `cargo fmt --all` completes without changes
- ✅ `cargo clippy` passes with no warnings (treated as errors with `-D warnings`)
- ✅ `cargo test --workspace` passes all tests
- ✅ `cargo build --workspace` builds successfully

**Why This Matters:**

- Prevents CI failures from basic issues
- Ensures code quality before commits
- Catches errors early in development cycle
- Maintains clean commit history

**Failure Handling:**

- If any command fails, fix the issues before committing
- Do not commit with failing tests or warnings
- Do not use `--no-verify` to bypass checks

#### Example Workflow

```bash
# Good: Atomic commits
git commit -m "test(mcp): add tests for ./rules/... path support"
git commit -m "fix(mcp): handle ./prefix in validate_rules_path"

# Bad: Mixed concerns
git commit -m "add tests and fix bug and refactor code"
```

---

### 2.5 Optional MCP Code Review

**Claude may offer code review before commits using MCP tools.**

#### When Claude Offers Review

Claude should offer MCP code review:

- **After** pre-commit verification passes (fmt/clippy/test/build)
- **Before** creating the commit
- For **non-trivial changes** (>50 lines modified)
- For **code changes** (skip docs-only, formatting-only)

Claude should **not** offer review for:

- Changes ≤50 lines
- Documentation-only changes (*.md files only)
- Formatting-only commits
- Commits explicitly marked as "WIP"

#### Review Offer Prompt

**Language Detection:**

Claude should detect the user's language from recent conversation messages and offer the review prompt in that language.

**English (default):**

> "Pre-commit verification passed ✅
>
> Would you like me to run an MCP code review before committing?
>
> This review will check:
>
> - TDD compliance (tests written first)
> - Rust safety (Path/PathBuf usage, no unwrap/panic)
> - Code quality (file/function size, error handling)
> - Cross-platform compatibility
> - Documentation completeness
> - Adherence to AGENTS.md guidelines
>
> Reply 'yes' to review, 'no' to skip."

**Japanese (日本語):**

> "Pre-commit verification passed ✅
>
> コミット前にMCPコードレビューを実行しますか？
>
> レビュー内容：
>
> - TDDコンプライアンス（テストファースト）
> - Rustの安全性（Path/PathBuf使用、unwrap/panicなし）
> - コード品質（ファイル・関数サイズ、エラー処理）
> - クロスプラットフォーム互換性
> - ドキュメント完全性
> - AGENTS.mdガイドライン準拠
>
> レビューする場合は 'yes'、スキップする場合は 'no' と返信してください。"

**Chinese (中文):**

> "Pre-commit verification passed ✅
>
> 是否在提交前运行MCP代码审查？
>
> 审查内容：
>
> - TDD合规性（测试优先）
> - Rust安全性（使用Path/PathBuf，无unwrap/panic）
> - 代码质量（文件/函数大小、错误处理）
> - 跨平台兼容性
> - 文档完整性
> - 遵守AGENTS.md指南
>
> 回复 'yes' 进行审查，'no' 跳过。"

**Other languages:**

Claude should translate the prompt to match the user's language while maintaining the same structure and checklist items.

#### MCP Review Process

If user approves review:

1. **Gather staged changes**:

   ```bash
   git diff --cached
   ```

2. **Invoke MCP review** using `mcp__codex__codex` tool:

   ```text
   Prompt: "Code review for typstlab commit.

   Review these changes against AGENTS.md guidelines:

   Files changed: [list]

   [git diff output]

   Check:
   1. TDD - Tests written before implementation?
   2. Rust Safety - Path/PathBuf used (not String)?
   3. Error Handling - No unwrap()/panic() in library code?
   4. Size Limits - Files <800 lines, functions <40 lines?
   5. Cross-Platform - Path components, not string manipulation?
   6. Documentation - Public APIs documented?
   7. Conventional Commits - Format correct?

   Provide:
   - Issues found (severity: error/warning/info)
   - Line numbers
   - Suggested fixes
   - Overall: approve | approve-with-warnings | needs-work"
   ```

3. **Present results to user**:
   - Show issues by severity
   - Highlight specific lines
   - Provide suggestions
   - Ask: "Fix issues / Commit anyway / Abort?"

4. **Respect user decision**:
   - "fix" → Help address issues
   - "commit" → Proceed with commit
   - "abort" → Cancel commit

#### Manual Review Request

User can request review anytime:

> User: "review this code"
> Claude: [Runs MCP review even for small changes]

#### Handling MCP Failures

If MCP service is unavailable:

> "MCP code review service is unavailable. Would you like to:
>
> - Commit without review (proceed)
> - Abort commit (abort)"

If MCP times out (>60s):

> "Review timed out. Would you like to:
>
> - Commit without review (proceed)
> - Abort commit (abort)"

#### Important Notes

- **Review is advisory, not blocking**: User can always commit anyway
- **Review does not replace mandatory checks**: fmt/clippy/test/build still required
- **Review results are not stored**: Each commit reviewed fresh
- **Only during typstlab development**: This is a workflow tool, not a product feature

---

### 3. Embrace Rust Safety and Cross-Platform Compatibility

**Use Rust's type system and standard library to guarantee correctness.**

#### Path Handling

**ALWAYS use `std::path::Path` and `std::path::PathBuf` for cross-platform compatibility.**

```rust
// ✅ Good: Cross-platform
use std::path::{Path, PathBuf};

let path = Path::new("papers").join("paper1").join("rules").join("guide.md");

// ❌ Bad: Platform-specific
let path = "papers/paper1/rules/guide.md";  // Breaks on Windows
```

#### Component Handling

```rust
// ✅ Good: Use Component enum
use std::path::Component;

let components: Vec<Component> = path.components().collect();
if components.iter().any(|c| matches!(c, Component::ParentDir)) {
    return Err(PathTraversalError);
}

// ❌ Bad: String manipulation
if path.contains("..") {  // Fragile, misses edge cases
    return Err(PathTraversalError);
}
```

#### Type Safety

```rust
// ✅ Good: Use newtypes for clarity
pub struct PaperId(String);
pub struct RulesPath(PathBuf);

// ❌ Bad: Primitive obsession
fn validate(paper_id: String, path: String) { /* ... */ }
```

#### Error Handling

```rust
// ✅ Good: Use Result with descriptive errors
fn process() -> Result<Output, TypstlabError> {
    let file = fs::read_to_string(path)
        .map_err(|e| TypstlabError::FileRead { path, source: e })?;
    Ok(parse(file)?)
}

// ❌ Bad: Panic or unwrap in library code
fn process() -> Output {
    let file = fs::read_to_string(path).unwrap();  // Never do this
    parse(file).unwrap()
}
```

#### Platform Testing

**All path-related code must have cross-platform tests:**

```rust
#[test]
fn test_cross_platform_path_handling() {
    let path = PathBuf::from("papers")
        .join("paper1")
        .join("rules")
        .join("guide.md");

    let result = validate_path(&path);
    assert!(result.is_ok(), "Should work on all platforms");
}
```

#### Path Validation and Security

**CRITICAL: Never use `Path::is_absolute()` directly for security validation.**

##### The Platform Semantics Problem

Rust's `std::path::Path::is_absolute()` has hidden platform-dependent behavior:

- Unix: `Path::new("/tmp").is_absolute()` → `true`
- Windows: `Path::new("/tmp").is_absolute()` → `false` (!)

**Result:** Security checks pass on macOS, fail silently on Windows.

**Real-world impact:** Bug discovered in Phase 2 of architectural review:

- Tests passed on macOS: `/tmp/malicious` caught by `is_absolute()`
- Tests failed on Windows CI: `/tmp/malicious` NOT caught by `is_absolute()`
- Error messages differed between platforms ("absolute path" vs "root directory")

##### Required Abstraction

**ALWAYS use `typstlab_core::path` utilities:**

```rust
use typstlab_core::path::has_absolute_or_rooted_component;

// ✅ Good: Cross-platform
if has_absolute_or_rooted_component(path) {
    bail!("Path cannot be absolute or rooted");
}

// ❌ Bad: Platform-dependent
if path.is_absolute() {
    bail!("Path cannot be absolute");
}
```

**Why this works:**

- Component-based analysis (universal across platforms)
- Detects both Unix absolute (`/tmp`) AND Windows rooted (`/tmp`) paths
- Same behavior on all platforms

**Testing strategy:**

```rust
#[test]
fn test_windows_rooted_detected_on_all_platforms() {
    // THE CRITICAL TEST: Must pass on macOS to catch Windows bugs
    let path = Path::new("/tmp");

    // Verify Component structure (universal)
    let components: Vec<Component> = path.components().collect();
    assert!(matches!(components[0], Component::RootDir));

    // Verify abstraction catches it (universal)
    assert!(has_absolute_or_rooted_component(path));
}
```

##### Code Review Checklist Addition

When reviewing path validation code:

- [ ] No direct use of `Path::is_absolute()` for security checks
- [ ] Use `has_absolute_or_rooted_component()` for validation
- [ ] Component-based tests verify Windows rooted paths
- [ ] Error messages mention "absolute or rooted" (not just "absolute")
- [ ] Tests include `/tmp` case (critical for Windows compatibility)

**See also:** `crates/typstlab-core/src/path/mod.rs` for implementation details.

---

### 4. Keep Files Readable and Focused

**Prioritize understanding over cleverness.**

#### File Size Guidelines

- **Target**: 300-500 lines per file
- **Warning**: 500-800 lines (consider splitting)
- **Action Required**: 800+ lines (must split into modules)

#### Module Organization

```rust
// ✅ Good: Focused modules
mod rules {
    mod validate;  // Path validation logic
    mod page;      // Pagination logic
    mod search;    // Search logic
}

// ❌ Bad: God file
// rules.rs with 2000+ lines mixing all concerns
```

#### Function Complexity

- **Target**: 10-20 lines per function
- **Warning**: 20-40 lines (consider extracting helpers)
- **Action Required**: 40+ lines (must refactor)

#### Code Organization

```rust
// ✅ Good: Single responsibility
fn validate_paper_id(components: &[Component]) -> Result<&OsStr> {
    match components.get(1) {
        Some(Component::Normal(name)) if !name.is_empty() => Ok(name),
        _ => Err(TypstlabError::InvalidPaperId),
    }
}

// ❌ Bad: Doing too much
fn validate_rules_path_and_read_and_parse_and_cache(...) {
    // 150 lines of mixed concerns
}
```

#### Documentation

```rust
// ✅ Good: Self-documenting with doc comments where needed
/// Validates that the requested path is within allowed directories.
///
/// # Security
///
/// - Blocks absolute paths
/// - Blocks parent directory traversal (`..`)
/// - Only allows `rules/` or `papers/<paper_id>/rules/`
///
/// # Examples
///
/// ```
/// let result = validate_rules_path(root, Path::new("rules/guide.md"));
/// assert!(result.is_ok());
/// ```
fn validate_rules_path(root: &Path, requested: &Path) -> Result<PathBuf> {
    // Implementation
}
```

---

### 5. DESIGN.md is the Source of Truth

**The specification lives in DESIGN.md. Implementation follows specification.**

#### Golden Rule

> **If implementation and DESIGN.md disagree, DESIGN.md is correct.**

#### When to Update DESIGN.md

**Must update DESIGN.md when:**
1. Changing API contracts
2. Modifying directory structure requirements
3. Altering command behavior
4. Changing schema definitions
5. Adjusting security policies

**May skip DESIGN.md updates for:**
- Internal refactoring (same public API)
- Performance optimizations (same behavior)
- Bug fixes that restore intended behavior
- Test additions

#### Update Process

**IMPORTANT: Do not update DESIGN.md optimistically.**

1. **Identify specification change**: "This behavior contradicts DESIGN.md section X"
2. **Discuss with developer**: "Should we update the spec or fix the implementation?"
3. **Get explicit approval**: "Confirm: Update DESIGN.md to allow Y?"
4. **Update atomically**: Spec change + implementation change in same commit

#### Example

```bash
# ❌ Bad: Update spec without discussion
git commit -m "feat: add new flag --force
Update DESIGN.md to document --force flag"

# ✅ Good: Discuss first
# 1. Identify: "DESIGN.md doesn't mention --force flag"
# 2. Ask: "Should I add --force flag? This requires DESIGN.md update."
# 3. Get approval: "Yes, add it"
# 4. Commit both:
git commit -m "feat: add --force flag for overriding safety checks

Update DESIGN.md section 5.2.3 to document new --force flag behavior.
This is a breaking change in behavior specification."
```

#### Rationale for Conservative Updates

**Why not update DESIGN.md freely?**

1. **Specification stability**: Frequent changes make specs unreliable
2. **Intentionality**: Changes should be deliberate, not accidental
3. **Coordination**: Other developers depend on stable contracts
4. **Versioning**: Spec changes may require version bumps

**DESIGN.md is not a living document that tracks implementation. Implementation tracks DESIGN.md.**

---

## Development Workflow

### Standard Workflow

```bash
# 1. Understand requirement
# - Read DESIGN.md section relevant to the task
# - Understand existing patterns in codebase

# 2. Write tests (TDD Red Phase)
# - Create test file or add to existing test module
# - Write failing tests that specify desired behavior
cargo test test_new_feature  # Verify it fails

# 3. Implement (TDD Green Phase)
# - Write minimal implementation
# - Use Rust safety features (Path, Component, Result)
# - Keep functions focused and files readable
cargo test test_new_feature  # Verify it passes

# 4. Verify (TDD Green Phase)
cargo test --workspace  # Ensure no regressions

# 5. Refactor (TDD Refactor Phase)
# - Improve code quality
# - Extract long functions
# - Add documentation
cargo test --workspace  # Keep tests green

# 6. Pre-Commit Verification (Mandatory)
cargo fmt --all                                      # Format code
cargo clippy --workspace --all-targets -- -D warnings  # Check for warnings
cargo test --workspace                                # Run all tests
cargo build --workspace                               # Verify build succeeds

# 7. Optional MCP Code Review
# Claude asks: "Would you like me to run an MCP code review?"
# - If yes: Run mcp__codex__codex with review prompt
# - Present results and get user decision
# - Skip for: ≤50 lines, docs-only, formatting-only

# 8. Commit (Atomic Unit)
git add <files>
git commit -m "type(scope): description"

# 9. Update DESIGN.md if needed (with approval)
# - Only if specification changed
# - Get explicit developer approval
# - Commit spec and implementation together
```

### Example: Adding a New Feature

**Task**: Add support for `.env` file validation

```bash
# Step 1: Check DESIGN.md
# - Does DESIGN.md mention .env files? No
# - Decision needed: Ask developer if this should be in spec

# Step 2: Write tests first (TDD)
cat > tests/env_validation_tests.rs <<EOF
#[test]
fn test_env_file_validation() {
    let result = validate_env_file(".env.example");
    assert!(result.is_ok());
}

#[test]
fn test_env_file_sensitive_blocked() {
    let result = validate_env_file(".env");
    assert!(result.is_err(), "Should block .env (contains secrets)");
}
EOF

cargo test test_env_file_validation  # Fails (good!)

# Step 3: Implement
# - Use Path for cross-platform compatibility
# - Add descriptive errors
# - Keep function focused

# Step 4: Tests pass
cargo test test_env_file_validation  # Passes
cargo test --workspace  # No regressions

# Step 5: Commit
git commit -m "feat(core): add .env file validation

Add validation for environment files:
- Allow .env.example (no secrets)
- Block .env (may contain secrets)
- Use Path API for cross-platform compatibility

Tests:
- test_env_file_validation
- test_env_file_sensitive_blocked"

# Step 6: Update DESIGN.md (if needed)
# - Ask developer: "Should I add .env handling to DESIGN.md section X?"
# - If yes: Update spec, commit separately or together
```

---

## Anti-Patterns to Avoid

### ❌ Don't: Skip TDD

```rust
// Bad: Implement first, test later (or never)
fn new_feature() -> Result<()> {
    // 100 lines of untested code
}
```

### ❌ Don't: Use String for Paths

```rust
// Bad: Platform-specific, error-prone
fn validate(path: String) -> bool {
    path.starts_with("rules/")  // Breaks on Windows
}

// Good: Cross-platform
fn validate(path: &Path) -> Result<()> {
    let components: Vec<_> = path.components().collect();
    // ...
}
```

### ❌ Don't: Write God Functions

```rust
// Bad: 200-line function doing everything
fn handle_request(input: Input) -> Output {
    // validation
    // parsing
    // business logic
    // error handling
    // formatting
    // caching
    // logging
}

// Good: Single Responsibility Principle
fn handle_request(input: Input) -> Result<Output> {
    let validated = validate_input(input)?;
    let parsed = parse_request(validated)?;
    let result = process_request(parsed)?;
    format_output(result)
}
```

### ❌ Don't: Update DESIGN.md Without Approval

```rust
// Bad: Silently change specification
// "I'll just add this flag to DESIGN.md..."

// Good: Explicit discussion
// "Should I add this flag? It requires updating DESIGN.md section 5.2."
```

### ❌ Don't: Use `unwrap()` in Library Code

```rust
// Bad: Panic in library code
let file = fs::read_to_string(path).unwrap();

// Good: Return Result
let file = fs::read_to_string(path)
    .map_err(|e| TypstlabError::FileRead { path, source: e })?;
```

---

## Testing Standards

### Test Coverage Requirements

- **New features**: 100% test coverage required
- **Bug fixes**: Add regression test before fixing
- **Refactoring**: Existing tests must pass

### Test Categories

```rust
#[cfg(test)]
mod security_tests {
    // Path traversal, injection, etc.
}

#[cfg(test)]
mod correctness_tests {
    // Business logic, edge cases
}

#[cfg(test)]
mod bounds_tests {
    // Boundary conditions, off-by-one
}

#[cfg(test)]
mod integration_tests {
    // End-to-end workflows
}
```

### Test Naming Convention

```rust
#[test]
fn test_<feature>_<scenario>_<expected>() {
    // Example: test_empty_file_with_cursor_1_allowed()
}
```

### Cross-Platform Tests

**All path-related tests must use TempDir and Path API:**

```rust
#[test]
fn test_cross_platform() {
    use tempfile::TempDir;
    use std::path::PathBuf;

    let temp = TempDir::new().unwrap();
    let path = temp.path().join("rules").join("guide.md");

    // Test implementation
}
```

---

## Code Review Checklist

Before considering work complete, verify:

- [ ] Tests written before implementation (TDD)
- [ ] All tests pass: `cargo test --workspace`
- [ ] MCP code review run (if change >50 lines)
- [ ] Cross-platform path handling used (`Path`/`PathBuf`)
- [ ] No files exceed 800 lines
- [ ] No functions exceed 40 lines
- [ ] Commit message follows Conventional Commits
- [ ] DESIGN.md updated if specification changed (with approval)
- [ ] No `unwrap()` or `panic!()` in library code
- [ ] Descriptive error types used (not `String`)
- [ ] Documentation added for public APIs

---

## When to Ask for Help

AI assistant should ask the developer when:

1. **Specification ambiguity**: DESIGN.md is unclear or contradictory
2. **DESIGN.md update needed**: Implementation requires spec change
3. **Breaking change**: Proposed change affects public API
4. **Security implications**: Change affects path validation, permissions, etc.
5. **Architectural decision**: Multiple valid approaches exist
6. **Test failure**: Existing tests fail after changes

---

## References

- **Specification**: [DESIGN.md](DESIGN.md)
- **Testing Guide**: [TESTING.md](TESTING.md)
- **Project Status**: Run `cargo test --workspace` for current state

---

## Summary

| Principle | Rule | Enforcement |
|-----------|------|-------------|
| **TDD** | Tests first, then implementation | Mandatory for features/fixes |
| **Commits** | Atomic, complete units of work | Developer decides timing |
| **Safety** | Use Rust types, cross-platform Path | Mandatory, verified by tests |
| **Readability** | 300-500 lines/file, 10-20 lines/function | Split if exceeded |
| **Specification** | DESIGN.md is source of truth | Update only with approval |

**Remember**: These guidelines exist to ensure high-quality, maintainable code that works reliably across platforms and resists regressions. Follow them diligently.

---

**End of AGENTS.md**
