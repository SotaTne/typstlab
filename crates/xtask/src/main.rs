use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use jsonschema::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check JSON files for schema validity
    JsonCheck,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::JsonCheck => run_json_check()?,
    }

    Ok(())
}

fn run_json_check() -> Result<()> {
    let base_dir = Path::new("crates/typstlab-base/src/version_resolver_jsons");
    let schema_path = base_dir.join("typst_version_schema.json");

    let schema_content = fs::read_to_string(&schema_path)
        .with_context(|| format!("Failed to read schema at {}", schema_path.display()))?;
    let schema_json: Value = serde_json::from_str(&schema_content)?;
    let compiled_schema = jsonschema::options()
        .build(&schema_json)
        .map_err(|e| anyhow!("Invalid schema: {}", e))?;

    let mut errors = 0;
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            let filename = path.file_name().unwrap().to_string_lossy();
            if filename == "typst_version_schema.json" {
                continue;
            }

            if let Err(e) = validate_file(&path, &compiled_schema) {
                eprintln!("❌ {}: {}", filename, e);
                errors += 1;
            } else {
                println!("✅ {}", filename);
            }
        }
    }

    if errors > 0 {
        return Err(anyhow!("JSON check failed with {} errors", errors));
    }

    Ok(())
}

fn validate_file(path: &Path, schema: &Validator) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&content).context("Failed to parse JSON")?;

    if !schema.is_valid(&value) {
        let mut msg = String::from("Validation failed:\n");
        for error in schema.iter_errors(&value) {
            msg.push_str(&format!("  - {}: {}\n", error.instance_path(), error));
        }
        return Err(anyhow!(msg));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn setup_validator() -> Validator {
        let schema_json = json!({
            "type": "object",
            "properties": {
                "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" }
            },
            "required": ["version"]
        });
        jsonschema::options().build(&schema_json).unwrap()
    }

    #[test]
    fn test_validate_file_success() {
        let schema = setup_validator();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"version": "0.14.2"}}"#).unwrap();
        assert!(validate_file(file.path(), &schema).is_ok());
    }

    #[test]
    fn test_validate_file_failure() {
        let schema = setup_validator();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"version": "invalid"}}"#).unwrap();
        assert!(validate_file(file.path(), &schema).is_err());
    }

    #[test]
    fn test_validate_file_malformed() {
        let schema = setup_validator();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"version": "#).unwrap();
        let result = validate_file(file.path(), &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse JSON"));
    }
}
