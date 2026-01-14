//! Integration tests for Status check system

use typstlab_core::{
    project::Project,
    status::{engine::StatusEngine, schema::CheckStatus},
};
use typstlab_testkit::temp_dir_in_workspace;

/// Helper to create a complete project structure
fn create_complete_project(root: &std::path::Path, papers: Vec<&str>) {
    // Create typstlab.toml
    std::fs::write(
        root.join("typstlab.toml"),
        r#"
[project]
name = "test-project"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
    )
    .unwrap();

    // Create papers/ directory
    let papers_dir = root.join("papers");
    std::fs::create_dir(&papers_dir).unwrap();

    // Create each paper with complete structure
    for paper_id in papers {
        let paper_dir = papers_dir.join(paper_id);
        std::fs::create_dir(&paper_dir).unwrap();

        // Create paper.toml
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
                paper_id, paper_id
            ),
        )
        .unwrap();

        // Create main.typ
        std::fs::write(paper_dir.join("main.typ"), "// Main content").unwrap();

        // Create _generated/
        std::fs::create_dir(paper_dir.join("_generated")).unwrap();
    }

    // Create layouts/ directory
    std::fs::create_dir(root.join("layouts")).unwrap();

    // Create refs/ directory with .bib file
    let refs_dir = root.join("refs");
    std::fs::create_dir(&refs_dir).unwrap();
    std::fs::write(refs_dir.join("core.bib"), "@article{test}").unwrap();
}

#[test]
fn test_status_engine_complete_project() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create complete project structure
    create_complete_project(root, vec!["paper1", "paper2"]);

    // Load project
    let project = Project::load(root.to_path_buf()).unwrap();

    // Run status engine
    let engine = StatusEngine::new();
    let report = engine.run(&project, None);

    // Verify all 4 checks ran
    assert_eq!(report.checks.len(), 4);

    let check_names: Vec<&str> = report.checks.iter().map(|c| c.name.as_str()).collect();
    assert!(check_names.contains(&"environment"));
    assert!(check_names.contains(&"typst"));
    assert!(check_names.contains(&"build"));
    assert!(check_names.contains(&"refs"));

    // Verify overall status is Pass (complete project)
    assert_eq!(
        report.overall_status,
        CheckStatus::Pass,
        "Complete project should have Pass status"
    );
}

#[test]
fn test_status_engine_paper_filter() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project with multiple papers
    create_complete_project(root, vec!["paper1", "paper2"]);

    // Remove main.typ from paper2 to create an error
    let paper2_main = root.join("papers").join("paper2").join("main.typ");
    std::fs::remove_file(paper2_main).unwrap();

    // Load project
    let project = Project::load(root.to_path_buf()).unwrap();

    // Run status with --paper filter for paper1 only
    let engine = StatusEngine::new();
    let report = engine.run(&project, Some("paper1"));

    // Verify paper filter is recorded
    assert_eq!(report.paper_filter, Some("paper1".to_string()));

    // paper1 has main.typ, so build check should pass
    let build_check = report.checks.iter().find(|c| c.name == "build");
    assert!(build_check.is_some());
    assert_eq!(
        build_check.unwrap().status,
        CheckStatus::Pass,
        "paper1 should pass build check"
    );

    // Overall status should be Pass (only checking paper1)
    assert_eq!(report.overall_status, CheckStatus::Pass);
}

#[test]
fn test_status_engine_error_conditions() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project with errors
    std::fs::write(
        root.join("typstlab.toml"),
        r#"
[project]
name = "test-project"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
    )
    .unwrap();

    // Create papers/ but don't create paper structure
    std::fs::create_dir(root.join("papers")).unwrap();

    // Create a paper directory with paper.toml but NO main.typ
    let paper_dir = root.join("papers").join("broken-paper");
    std::fs::create_dir(&paper_dir).unwrap();
    std::fs::write(
        paper_dir.join("paper.toml"),
        r#"
[paper]
id = "broken-paper"
title = "Broken Paper"
language = "en"
date = "2026-01-14"

[output]
name = "broken-paper"
"#,
    )
    .unwrap();

    // Load project
    let project = Project::load(root.to_path_buf()).unwrap();

    // Run status engine
    let engine = StatusEngine::new();
    let report = engine.run(&project, None);

    // Verify errors detected
    assert_eq!(
        report.overall_status,
        CheckStatus::Error,
        "Project with missing main.typ should have Error status"
    );

    // Verify build check found the error
    let build_check = report.checks.iter().find(|c| c.name == "build");
    assert!(build_check.is_some());
    assert_eq!(build_check.unwrap().status, CheckStatus::Error);

    // Verify actions suggested
    assert!(
        !report.actions.is_empty(),
        "Should suggest actions to fix errors"
    );
}

#[test]
fn test_status_engine_warning_conditions() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project with warnings (missing optional items)
    std::fs::write(
        root.join("typstlab.toml"),
        r#"
[project]
name = "test-project"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
    )
    .unwrap();

    let papers_dir = root.join("papers");
    std::fs::create_dir(&papers_dir).unwrap();

    // Create paper with main.typ but no _generated/ and no layouts/
    let paper_dir = papers_dir.join("paper1");
    std::fs::create_dir(&paper_dir).unwrap();
    std::fs::write(
        paper_dir.join("paper.toml"),
        r#"
[paper]
id = "paper1"
title = "Test Paper"
language = "en"
date = "2026-01-14"

[output]
name = "paper1"
"#,
    )
    .unwrap();
    std::fs::write(paper_dir.join("main.typ"), "// Content").unwrap();

    // Don't create layouts/ or refs/ (optional)

    // Load project
    let project = Project::load(root.to_path_buf()).unwrap();

    // Run status engine
    let engine = StatusEngine::new();
    let report = engine.run(&project, None);

    // Verify warnings detected (missing optional items)
    assert_eq!(
        report.overall_status,
        CheckStatus::Warning,
        "Project with missing optional items should have Warning status"
    );

    // Verify environment check has warning (no layouts/)
    let env_check = report.checks.iter().find(|c| c.name == "environment");
    assert!(env_check.is_some());
    assert_eq!(env_check.unwrap().status, CheckStatus::Warning);

    // Verify refs check has warning (no refs/)
    let refs_check = report.checks.iter().find(|c| c.name == "refs");
    assert!(refs_check.is_some());
    assert_eq!(refs_check.unwrap().status, CheckStatus::Warning);
}
