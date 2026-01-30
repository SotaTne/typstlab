pub mod layout;
pub mod lib;
pub mod paper;

use crate::cli::GenCommands;
use anyhow::Result;

pub fn run(command: GenCommands, verbose: bool) -> Result<()> {
    match command {
        GenCommands::Paper { id, layout, title } => paper::run(id, layout, title, verbose),
        GenCommands::Layout { name } => layout::run(name, verbose),
        GenCommands::Lib { name } => lib::run(name, verbose),
    }
}
