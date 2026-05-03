use anyhow::{Result, anyhow};
use colored::Colorize;
use std::path::Path;
use typstlab_app::{AppContext, StatusAction, StatusError, StatusOutput, StatusWarning};
use typstlab_proto::{Action, CliSpeaker};

pub fn run(ctx: AppContext, _verbose: bool) -> Result<()> {
    let action = StatusAction::new(ctx.loaded_project, ctx.toolchain);
    let presenter = StatusPresenter;
    let mut warnings = Vec::new();

    match action.run(&mut |_| {}, &mut |warning| warnings.push(warning)) {
        Ok(output) => {
            presenter.render_result(&output);
            for warning in warnings {
                presenter.render_warning(warning);
            }
            Ok(())
        }
        Err(errors) => {
            for error in &errors {
                presenter.render_error(error);
            }
            Err(anyhow!("Status failed"))
        }
    }
}

struct StatusPresenter;

impl CliSpeaker for StatusPresenter {
    type Event = ();
    type Warning = StatusWarning;
    type Error = StatusError;
    type Output = StatusOutput;

    fn render_event(&self, _event: typstlab_proto::AppEvent<Self::Event>) {}

    fn render_warning(&self, warning: Self::Warning) {
        let (label, path) = match warning {
            StatusWarning::PapersDirNotFound(path) => ("papers directory missing", path),
            StatusWarning::TemplatesDirNotFound(path) => ("templates directory missing", path),
            StatusWarning::DistDirNotFound(path) => ("dist directory missing", path),
        };

        eprintln!(
            "{} {}: {}",
            "⚠".yellow(),
            label.yellow(),
            path.display().to_string().dimmed()
        );
    }

    fn render_error(&self, error: &Self::Error) {
        eprintln!("{} {}", "❌ Status failed:".red().bold(), error);
    }

    fn render_result(&self, output: &Self::Output) {
        println!("{}", "typstlab status".bright_blue().bold());
        println!();

        print_section("Project");
        print_value("name", &output.project.name);
        print_path("root", &output.project.root_path);
        println!();

        print_section("Toolchain");
        print_resource(
            "typst",
            &[("version", output.toolchain.typst.version.as_str())],
            "binary",
            &output.toolchain.typst.path_in_store,
        );
        if let Some(docs) = &output.docs {
            print_resource(
                "docs",
                &[("version", output.toolchain.typst.version.as_str())],
                "path",
                &docs.path_in_store,
            );
            print_path_indented(4, "cache", &docs.cache);
        }
        println!();

        print_collection("Papers", &output.papers.root, &output.papers.items);
        println!();
        print_collection("Templates", &output.templates.root, &output.templates.items);
        println!();

        print_section("Dist");
        print_path("dist", &output.dist.root);
    }
}

fn print_section(title: &str) {
    println!("{}", title.bright_blue().bold());
}

fn print_value(label: &str, value: &str) {
    println!("  {:<8} {}", label.bright_black(), value.bold());
}

fn print_path(label: &str, path: &Path) {
    print_path_indented(2, label, path);
}

fn print_path_indented(indent: usize, label: &str, path: &Path) {
    let padding = " ".repeat(indent);
    println!(
        "{}{:<8} {}",
        padding,
        label.bright_black(),
        path.display().to_string().bright_black()
    );
}

fn print_resource(title: &str, values: &[(&str, &str)], path_label: &str, path: &Path) {
    println!("  {}", title.green().bold());
    for (label, value) in values {
        println!("    {:<8} {}", label.bright_black(), value.bold());
    }
    println!(
        "    {:<8} {}",
        path_label.bright_black(),
        path.display().to_string().bright_black()
    );
}

fn print_collection(title: &str, root: &Path, items: &[String]) {
    let count = format!("{} item(s)", items.len());
    println!("{} {}", title.bright_blue().bold(), count.green().bold());

    if items.is_empty() {
        println!("  {}", "none".bright_black());
    } else {
        print_items(items);
    }

    print_path("path", root);
}

fn print_items(items: &[String]) {
    const MAX_LINE_WIDTH: usize = 88;
    let mut line = String::from("  ");
    let mut visible_len = 0;

    for item in items {
        let styled_item = item.bold().to_string();
        let separator_width = if visible_len == 0 { 0 } else { 2 };
        let next_len = visible_len + separator_width + item.len();

        if next_len > MAX_LINE_WIDTH && visible_len == 0 {
            println!("  {}", styled_item);
            continue;
        }

        if next_len > MAX_LINE_WIDTH {
            println!("{}", line);
            line.clear();
            line.push_str("  ");
            visible_len = 0;
        }

        if visible_len > 0 {
            line.push_str("  ");
            visible_len += 2;
        }
        line.push_str(&styled_item);
        visible_len += item.len();
    }

    if visible_len > 0 {
        println!("{}", line);
    }
}
