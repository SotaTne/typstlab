use typstlab_core::context::Context;
use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(name: String, verbose: bool) -> Result<()> {
    let ctx = Context::builder().verbose(verbose).build()?;

    let template_dir = ctx.project.as_ref().expect("Project not found").root.join("templates").join(&name);

    if template_dir.exists() {
        println!("{} Template '{}' already exists", "!".yellow(), name);
        return Ok(());
    }

    if verbose {
        println!("{} Creating template '{}'...", "→".cyan(), name);
    }

    fs::create_dir_all(&template_dir)?;

    // Create new style template files
    fs::write(
        template_dir.join("template.typ"),
        r#"// Custom template.typ
#let project(
  title: "",
  authors: (),
  date: none,
  language: "en",
  body
) = {
  set text(lang: language, size: 11pt)
  set page(paper: "a4", margin: 2cm)
  
  align(center)[
    #text(size: 1.5em, weight: "bold", title) \
    #v(1em)
    #grid(
        columns: (1fr,) * calc.min(3, authors.len()),
        ..authors.map(a => [#a.name \ #a.email])
    )
    #v(1em)
    #date
  ]
  
  body
}
"#,
    )?;

    fs::write(
        template_dir.join("main.tmp.typ"),
        r#"#import "template.typ": *

#show: project.with(
  title: "{{ paper.title }}",
  authors: (
    {{ each paper.authors |author| }}
    (name: "{{ author.name }}", email: "{{ author.email }}", affiliation: "{{ author.affiliation }}"),
    {{ /each }}
  ),
  date: "{{ paper.date }}",
  language: "{{ paper.language }}",
)

= Introduction
This is a new template based on the typstlab scaffold.
"#,
    )?;

    println!(
        "{} Created template '{}' at {}",
        "✓".green().bold(),
        name,
        template_dir.display()
    );

    Ok(())
}
