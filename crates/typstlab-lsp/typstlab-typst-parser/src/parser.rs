use tree_sitter::{Parser, Tree};
use tree_sitter_typstlabtypst::LANGUAGE;

#[allow(dead_code)]
pub fn parse_tree_sitter(src: &mut impl AsRef<[u8]>, old_tree: Option<&Tree>) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Typst grammar");
    parser.parse(src, old_tree)
}
