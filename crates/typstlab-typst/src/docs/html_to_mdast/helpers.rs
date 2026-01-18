//! Helper functions for HTML to mdast conversion

use markup5ever::Attribute;
use markup5ever_rcdom::{Handle, NodeData};
use std::cell::RefCell;

/// Gets class attribute value from element attributes
#[allow(dead_code)]
pub fn get_class(attrs: &RefCell<Vec<Attribute>>) -> Option<String> {
    attrs
        .borrow()
        .iter()
        .find(|attr| attr.name.local.as_ref() == "class")
        .map(|attr| attr.value.to_string())
}

/// Gets attribute value by name from element attributes
pub fn get_attr(attrs: &RefCell<Vec<Attribute>>, name: &str) -> Option<String> {
    attrs
        .borrow()
        .iter()
        .find(|attr| attr.name.local.as_ref() == name)
        .map(|attr| attr.value.to_string())
}

/// Collects all text content from children recursively
///
/// Flattens all text nodes within the element tree.
pub fn collect_text_from_children(handle: &Handle) -> String {
    let mut text = String::new();
    for child in handle.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                text.push_str(&contents.borrow());
            }
            NodeData::Element { .. } => {
                text.push_str(&collect_text_from_children(child));
            }
            _ => {}
        }
    }
    text
}
