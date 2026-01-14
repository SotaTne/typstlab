//! References check - validates bibliography setup
//!
//! NOTE: v0.2で完全実装予定。v0.1ではスタブとして基本チェックのみ実装。
//! v0.2では以下の機能を追加:
//! - refs/sets/ の構造検証
//! - sources.lock の検証
//! - DOI/URL フェッチ機能の統合

use crate::status::engine::{CheckContext, CheckResult, StatusCheck};

pub struct RefsCheck;

impl StatusCheck for RefsCheck {
    fn name(&self) -> &str {
        "refs"
    }

    fn run(&self, context: &CheckContext) -> CheckResult {
        let root = context.root();
        let refs_dir = root.join("refs");

        // Check if refs/ directory exists
        if !refs_dir.exists() {
            return CheckResult::warning("refs/ directory not found (optional feature)");
        }

        // Check for .bib files
        match std::fs::read_dir(&refs_dir) {
            Ok(entries) => {
                let has_bib_files = entries.filter_map(|e| e.ok()).any(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "bib")
                        .unwrap_or(false)
                });

                if !has_bib_files {
                    CheckResult::warning("No .bib files found in refs/ directory")
                } else {
                    CheckResult::pass()
                }
            }
            Err(_) => CheckResult::warning("Cannot read refs/ directory"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::Project, status::schema::CheckStatus};
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_refs_check_pass_with_bib_files() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project with refs/
        std::fs::write(
            root.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        std::fs::create_dir(root.join("papers")).unwrap();

        let refs_dir = root.join("refs");
        std::fs::create_dir(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("core.bib"), "@article{...}").unwrap();

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = RefsCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[test]
    fn test_refs_check_warning_no_refs_dir() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project without refs/
        std::fs::write(
            root.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        std::fs::create_dir(root.join("papers")).unwrap();

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = RefsCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result
            .messages
            .iter()
            .any(|m| m.contains("refs/ directory not found")));
    }

    #[test]
    fn test_refs_check_warning_no_bib_files() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project with empty refs/
        std::fs::write(
            root.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        std::fs::create_dir(root.join("papers")).unwrap();
        std::fs::create_dir(root.join("refs")).unwrap();

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = RefsCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result.messages.iter().any(|m| m.contains("No .bib files")));
    }
}
