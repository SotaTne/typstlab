use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use typstlab_base::docs_parser::DocsEntry;

pub fn run(files: &[PathBuf]) -> Result<()> {
    if files.is_empty() {
        return Err(anyhow!(
            "check_docs_schema requires at least one docs.json path"
        ));
    }

    let mut failures = Vec::new();
    for file in files {
        match validate_docs_json(file) {
            Ok(()) => println!("✅ {}", file.display()),
            Err(error) => {
                eprintln!("❌ {}\n{}", file.display(), error);
                failures.push(file.display().to_string());
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "docs schema check failed for {} file(s): {}",
            failures.len(),
            failures.join(", ")
        ))
    }
}

fn validate_docs_json(path: &Path) -> Result<()> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut deserializer = serde_json::Deserializer::from_reader(reader);

    serde_path_to_error::deserialize::<_, Vec<DocsEntry>>(&mut deserializer)
        .map(|_| ())
        .map_err(|error| {
            anyhow!(
                "path: {}\nerror: {}",
                format_json_path(&error.path().to_string()),
                error.inner()
            )
        })
}

fn format_json_path(path: &str) -> String {
    if path == "." {
        "$".to_string()
    } else {
        format!("${}", path.trim_start_matches('.'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_docs_json_success() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[
                {{
                    "route": "/DOCS-BASE/",
                    "title": "Overview",
                    "body": {{ "kind": "html", "content": "<p>Hello</p>" }}
                }}
            ]"#
        )
        .unwrap();

        validate_docs_json(file.path()).unwrap();
    }

    #[test]
    fn test_validate_docs_json_reports_path() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[
                {{
                    "route": "/DOCS-BASE/",
                    "title": "Overview",
                    "body": {{ "kind": "html", "content": {{ "not": "html" }} }}
                }}
            ]"#
        )
        .unwrap();

        let error = validate_docs_json(file.path()).unwrap_err().to_string();

        assert!(error.contains("path: $[0].body"));
        assert!(error.contains("invalid type"));
    }

    #[test]
    fn test_run_requires_files() {
        let error = run(&[]).unwrap_err().to_string();

        assert!(error.contains("requires at least one docs.json path"));
    }
}
