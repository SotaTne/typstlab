use assert_cmd::assert::OutputAssertExt;
use std::fs;
use typstlab_e2e_tests::{e2e_command, e2e_temp_dir};

#[test]
fn scenario_template_cloning_with_package() {
    let temp = e2e_temp_dir();
    let root = temp.path();

    // 1. Init project
    e2e_command(root)
        .arg("init")
        .assert()
        .success();

    // 2. Create a custom template with @preview package
    let template_dir = root.join("templates/gibz");
    fs::create_dir_all(&template_dir).unwrap();
    
    let main_tmp_content = r#"#import "@preview/gibz-script:0.1.0": gibz-script
#show: gibz-script.with(title: "{{ paper.title }}")

= Content
Hello from gibz template!
"#;
    fs::write(template_dir.join("main.tmp.typ"), main_tmp_content).unwrap();
    fs::write(template_dir.join("template.typ"), "// Common layout").unwrap();

    // 3. Generate paper using this template
    e2e_command(root)
        .arg("gen")
        .arg("paper")
        .arg("my-paper")
        .arg("-t")
        .arg("gibz")
        .assert()
        .success();

    // 4. Verify results
    let paper_dir = root.join("papers/my-paper");
    assert!(paper_dir.exists());
    
    // We expect main.tmp.typ to be renamed to main.typ and processed
    // If our current implementation doesn't do this, the test will fail here.
    let main_typ = paper_dir.join("main.typ");
    assert!(main_typ.exists(), "main.typ should exist (renamed from main.tmp.typ)");
    
    let content = fs::read_to_string(main_typ).unwrap();
    assert!(content.contains("@preview/gibz-script:0.1.0"));
    assert!(content.contains("my-paper")); // substitution check
    
    assert!(paper_dir.join("template.typ").exists());
}
