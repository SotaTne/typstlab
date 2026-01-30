//! Build structure check - validates paper structure

use crate::status::{
    engine::{CheckContext, CheckResult, StatusCheck},
    schema::{Action, Safety},
};

pub struct BuildCheck;

impl StatusCheck for BuildCheck {
    fn name(&self) -> &str {
        "build"
    }

    fn run(&self, context: &CheckContext) -> CheckResult {
        let papers = context.target_papers();

        if papers.is_empty() {
            return CheckResult::pass("build", "No papers found to check");
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
                actions.push(Action {
                    id: format!("create_main_{}", paper_id),
                    command: format!("touch papers/{}/main.typ", paper_id), // Naive command for now
                    description: format!("Create main entry file for paper '{}'", paper_id),
                    enabled: true,
                    disabled_reason: None,
                    safety: Safety {
                        network: false,
                        writes: true,
                        writes_sot: true,
                        reads: false,
                    },
                    prerequisite: None,
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
            let primary_message = "One or more papers missing main.typ".to_string();
            let mut result = CheckResult::error("build_structure", primary_message);

            // Add detailed messages
            // For now, let's join messages if they fit, or just use the summary and rely on "details".
            // Actually, let's put the list of issues in details.
            result = result.with_detail(
                "issues",
                serde_json::Value::Array(
                    messages
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );

            for action in actions {
                result = result.with_action(action);
            }
            result
        } else if has_warning {
            CheckResult::warning(
                "build_structure",
                "Some papers missing _generated/ directory",
            )
        } else {
            CheckResult::pass("build_structure", "All papers structurally valid")
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
        // Message is generic summary
        assert!(result
            .message
            .contains("One or more papers missing main.typ"));

        // Detailed issues in details
        let details = result.details.expect("Should have details");
        let issues = details.get("issues").expect("Should have issues list");
        assert!(issues.as_array().unwrap().iter().any(|v| v
            .as_str()
            .unwrap()
            .contains("Paper 'paper1': main.typ not found")));

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
            .message
            .contains("Some papers missing _generated/ directory"));
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
