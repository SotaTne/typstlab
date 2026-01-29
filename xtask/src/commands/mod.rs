use anyhow::Result;

pub mod tree_sitter;
pub mod tree_sitter_verify;

pub trait Command {
    fn run(&self) -> Result<()>;
}
