//! Build structure check - validates paper structure

use crate::status::{
    engine::{CheckContext, CheckResult, StatusCheck},
    schema::SuggestedAction,
};

pub struct BuildCheck;

impl StatusCheck for BuildCheck {
    fn name(&self) -> &str {
        "build"
    }

    fn run(&self, context: &CheckContext) -> CheckResult {
        let papers = context.target_papers();

        if papers.is_empty() {
            return CheckResult::pass();
        }

        let mut messages = Vec::new();
        let mut actions = Vec::new();
        let mut has_error = false;
        let mut has_warning = false;

        for paper in papers {
            let paper_id = paper.id();

            // Check main.typ exists (Error if missing)
            if !paper.has_main_file() {
                has_error = true;
                messages.push(format!("Paper '{}': main.typ not found", paper_id));
                actions.push(SuggestedAction::CreateFile {
                    path: format!("papers/{}/main.typ", paper_id),
                    description: format!("Create main entry file for paper '{}'", paper_id),
                });
            }

            // Check _generated/ exists (Warning if missing)
            let generated_dir = paper.generated_dir();
            if !generated_dir.exists() {
                has_warning = true;
                messages.push(format!(
                    "Paper '{}': _generated/ directory not found (optional)",
                    paper_id
                ));
            }
        }

        // Return result based on findings
        if has_error {
            let mut result =
                CheckResult::error("One or more papers missing main.typ").with_messages(messages);
            for action in actions {
                result = result.with_action(action);
            }
            result
        } else if has_warning {
            CheckResult::warning("Some papers missing _generated/ directory")
                .with_messages(messages)
        } else {
            CheckResult::pass()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::Project, status::schema::CheckStatus};
    use typstlab_testkit::temp_dir_in_workspace;

    fn create_paper(root: &std::path::Path, id: &str, with_main: bool, with_generated: bool) {
        let paper_dir = root.join("papers").join(id);
        std::fs::create_dir_all(&paper_dir).unwrap();

        std::fs::write(
            paper_dir.join("paper.toml"),
            format!(
                r#"
[paper]
id = "{}"
title = "Test Paper"
language = "en"
date = "2026-01-14"

[output]
name = "{}"
"#,
                id, id
            ),
        )
        .unwrap();

        if with_main {
            std::fs::write(paper_dir.join("main.typ"), "// Main file").unwrap();
        }

        if with_generated {
            std::fs::create_dir(paper_dir.join("_generated")).unwrap();
        }
    }

    #[test]
    fn test_build_check_pass_with_main_typ() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project
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
        create_paper(root, "paper1", true, true);

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = BuildCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[test]
    fn test_build_check_error_missing_main_typ() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

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
        create_paper(root, "paper1", false, true); // No main.typ

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = BuildCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Error);
        assert!(result
            .messages
            .iter()
            .any(|m| m.contains("main.typ not found")));
        assert!(!result.actions.is_empty());
    }

    #[test]
    fn test_build_check_warning_missing_generated_dir() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

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
        create_paper(root, "paper1", true, false); // No _generated/

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = BuildCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result
            .messages
            .iter()
            .any(|m| m.contains("_generated/ directory not found")));
    }

    #[test]
    fn test_build_check_with_paper_filter() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

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
        create_paper(root, "paper1", true, true);
        create_paper(root, "paper2", false, true); // paper2 missing main.typ

        let project = Project::load(root.to_path_buf()).unwrap();

        // Check only paper1 (should pass)
        let context = CheckContext {
            project: &project,
            target_paper: Some("paper1"),
        };

        let check = BuildCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Pass);
    }
}
