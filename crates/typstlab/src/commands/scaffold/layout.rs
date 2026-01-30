use crate::context::Context;
use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(name: String, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    let layout_dir = ctx.project.root.join("layouts").join(&name);

    if layout_dir.exists() {
        println!("{} Layout '{}' already exists", "!".yellow(), name);
        return Ok(());
    }

    if verbose {
        println!("{} Creating layout '{}'...", "→".cyan(), name);
    }

    fs::create_dir_all(&layout_dir)?;

    // Create basic template files
    fs::write(
        layout_dir.join("meta.tmp.typ"),
        r#"#let paper_meta = (
  id: "{{ ID }}",
  title: "{{ TITLE }}",
  authors: (
    {{ AUTHORS }}
  ),
  date: datetime(
    year: {{ YEAR }},
    month: {{ MONTH }},
    day: {{ DAY }}
  ),
  language: "{{ LANGUAGE }}",
)
"#,
    )?;

    fs::write(
        layout_dir.join("header.typ"),
        r#"// Document configuration
#let setup(body) = {
  set page(paper: "a4")
  set text(font: "Linux Libertine", size: 11pt)
  body
}
"#,
    )?;

    fs::write(
        layout_dir.join("refs.tmp.typ"),
        r#"// Bibliography configuration
#let bibliography = bibliography("{{ REFS_PATH }}")
"#,
    )?;

    println!(
        "{} Created layout '{}' at {}",
        "✓".green().bold(),
        name,
        layout_dir.display()
    );

    Ok(())
}
