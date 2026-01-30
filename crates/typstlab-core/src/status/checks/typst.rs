//! Typst installation check
//!
//! NOTE: This is a skeleton implementation. Full Typst resolution logic
//! will be implemented in the Typst management phase.

use crate::status::engine::{CheckContext, CheckResult, StatusCheck};

pub struct TypstCheck;

impl StatusCheck for TypstCheck {
    fn name(&self) -> &str {
        "typst"
    }

    fn run(&self, _context: &CheckContext) -> CheckResult {
        // Skeleton implementation: Always pass
        // Full Typst resolution and version checking will be implemented
        // in the Typst management phase (Phase 2 of overall plan)
        CheckResult::pass("typst_available", "Typst check passed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::Project, status::schema::CheckStatus};
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_typst_check_skeleton_always_passes() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create minimal project
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

        let check = TypstCheck;
        let result = check.run(&context);

        // Skeleton implementation always passes
        assert_eq!(result.status, CheckStatus::Pass);
    }
}
