//! Environment check - validates directory structure

use crate::status::{
    engine::{CheckContext, CheckResult, StatusCheck},
    schema::{Action, CheckStatus, Safety},
};

pub struct EnvCheck;

impl StatusCheck for EnvCheck {
    fn name(&self) -> &str {
        "environment"
    }

    fn run(&self, context: &CheckContext) -> CheckResult {
        let root = context.root();
        let mut messages = Vec::new();
        let mut actions = Vec::new();
        let mut has_error = false;
        let mut has_warning = false;

        // Check typstlab.toml exists (should always exist if we loaded the project)
        let config_path = root.join("typstlab.toml");
        if !config_path.exists() {
            has_error = true;
            messages.push("typstlab.toml not found".to_string());
            actions.push(Action {
                id: "init_project".to_string(),
                command: "typstlab init".to_string(),
                description: "Initialize new project".to_string(),
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

        // Check papers/ directory exists (required)
        let papers_dir = root.join("papers");
        if !papers_dir.exists() {
            has_error = true;
            messages.push("papers/ directory not found".to_string());
            actions.push(Action {
                id: "create_papers_dir".to_string(),
                command: "mkdir papers".to_string(),
                description: "Create papers directory".to_string(),
                enabled: true,
                disabled_reason: None,
                safety: Safety {
                    network: false,
                    writes: true,
                    writes_sot: false,
                    reads: false,
                },
                prerequisite: None,
            });
        }

        // Check layouts/ directory exists (optional)
        let layouts_dir = root.join("layouts");
        if !layouts_dir.exists() {
            has_warning = true;
            messages.push("layouts/ directory not found (optional)".to_string());
        }

        // Return result based on findings
        if has_error {
            let primary_message = messages.join(", ");
            let mut result = CheckResult::error("project_structure", primary_message);
            for action in actions {
                result = result.with_action(action);
            }
            result
        } else if has_warning {
            let primary_message = messages.join(", ");
            CheckResult::warning("project_structure", primary_message)
        } else {
            CheckResult::pass("project_structure", "All required directories present")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::Project, status::schema::CheckStatus};
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_env_check_pass_complete_structure() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create complete project structure
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
        std::fs::create_dir(root.join("layouts")).unwrap();

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = EnvCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Pass);
        assert_eq!(result.message, "All required directories present");
    }

    #[test]
    fn test_env_check_error_missing_papers_dir() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project without papers/ directory
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

        // Don't create papers/ directory
        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = EnvCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Error);
        assert!(result.message.contains("papers/ directory not found"));
        assert!(!result.actions.is_empty());
    }

    #[test]
    fn test_env_check_warning_missing_layouts_dir() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project with papers/ but without layouts/
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
        // Don't create layouts/ directory

        let project = Project::load(root.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let check = EnvCheck;
        let result = check.run(&context);

        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result.message.contains("layouts/ directory not found"));
    }
}
