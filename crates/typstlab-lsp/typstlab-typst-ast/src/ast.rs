use std::collections::HashMap;
use std::path::PathBuf;
use tree_sitter::Tree;

pub struct Programs {
    path: HashMap<PathBuf, Program>,
}

pub struct Program {
    path: PathBuf,
    text: String,
    tree: Box<Tree>,
}

enum Node {}
