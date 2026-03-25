pub mod docs;
pub mod exec;
pub mod install;
pub mod util;
pub mod version;
pub mod versions;

use crate::cli::{DocsCommands, TypstSubcommands};
use anyhow::Result;

pub fn run(args: TypstSubcommands, verbose: bool) -> Result<()> {
    match args {
        TypstSubcommands::Install { version } => install::execute_install(version),
        TypstSubcommands::Version { json } => version::execute_version(json),
        TypstSubcommands::Versions { json } => versions::execute_versions(json),
        TypstSubcommands::Exec { args } => exec::execute_exec(args),
        TypstSubcommands::Docs(docs_cmd) => match docs_cmd {
            DocsCommands::Sync => docs::sync(verbose),
            DocsCommands::Clear => docs::clear(verbose),
            DocsCommands::Status { json } => docs::status(json, verbose),
        },
    }
}
