pub mod lib;
pub mod paper;
pub mod template;

use crate::cli::GenCommands;
use anyhow::Result;

pub fn run(command: GenCommands, verbose: bool) -> Result<()> {
    match command {
        GenCommands::Paper {
            id,
            template,
            title,
        } => paper::run(id, template, title, verbose),
        GenCommands::Template { name } => template::run(name, verbose),
        GenCommands::Lib { name } => lib::run(name, verbose),
    }
}
