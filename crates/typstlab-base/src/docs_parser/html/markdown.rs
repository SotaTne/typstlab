use super::ast::{HtmlElement, HtmlNode, HtmlTag, HtmlTree};
use super::error::HtmlToMarkdownError;
use crate::docs_parser::md::{
    Block, Document, Inline, ListItem, Table, TableAlign, TableCell, TableRow,
};
use crate::docs_parser::route::resolve_docs_href;

#[derive(Debug, Clone, Default)]
pub struct MarkdownContext {
    pub source_route: Option<String>,
}

pub trait ToMarkdownDocument {
    fn to_markdown_blocks(&self) -> Result<Vec<Block>, HtmlToMarkdownError> {
        self.to_markdown_blocks_with_context(&MarkdownContext::default())
    }

    fn to_markdown_blocks_with_context(
        &self,
        context: &MarkdownContext,
    ) -> Result<Vec<Block>, HtmlToMarkdownError>;

    fn to_markdown_document(&self) -> Result<Document, HtmlToMarkdownError> {
        self.to_markdown_document_with_context(&MarkdownContext::default())
    }

    fn to_markdown_document_with_context(
        &self,
        context: &MarkdownContext,
    ) -> Result<Document, HtmlToMarkdownError> {
        Ok(Document::new(
            self.to_markdown_blocks_with_context(context)?,
        ))
    }
}

pub fn html_tree_to_markdown_document(tree: &HtmlTree) -> Result<Document, HtmlToMarkdownError> {
    tree.to_markdown_document()
}

pub fn html_tree_to_markdown_document_with_context(
    tree: &HtmlTree,
    context: &MarkdownContext,
) -> Result<Document, HtmlToMarkdownError> {
    tree.to_markdown_document_with_context(context)
}

impl ToMarkdownDocument for HtmlTree {
    fn to_markdown_blocks_with_context(
        &self,
        context: &MarkdownContext,
    ) -> Result<Vec<Block>, HtmlToMarkdownError> {
        nodes_to_blocks(&self.children, context)
    }
}

impl ToMarkdownDocument for HtmlNode {
    fn to_markdown_blocks_with_context(
        &self,
        context: &MarkdownContext,
    ) -> Result<Vec<Block>, HtmlToMarkdownError> {
        match self {
            Self::Text(value) => {
                if value.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(vec![Block::Paragraph(vec![Inline::Text(value.clone())])])
                }
            }
            Self::Element(element) => element.to_markdown_blocks_with_context(context),
        }
    }
}

impl ToMarkdownDocument for HtmlElement {
    fn to_markdown_blocks_with_context(
        &self,
        context: &MarkdownContext,
    ) -> Result<Vec<Block>, HtmlToMarkdownError> {
        let block = match self.tag {
            HtmlTag::H1 => heading(1, children_to_inlines(&self.children, context)?),
            HtmlTag::H2 => heading(2, children_to_inlines(&self.children, context)?),
            HtmlTag::H3 => heading(3, children_to_inlines(&self.children, context)?),
            HtmlTag::H4 => heading(4, children_to_inlines(&self.children, context)?),
            HtmlTag::H5 => heading(5, children_to_inlines(&self.children, context)?),
            HtmlTag::H6 => heading(6, children_to_inlines(&self.children, context)?),
            HtmlTag::P => Block::Paragraph(children_to_inlines(&self.children, context)?),
            HtmlTag::Code => code_or_paragraph(text_content(&self.children)),
            HtmlTag::Pre => Block::Code {
                lang: None,
                value: text_content(&self.children),
            },
            HtmlTag::Ul => Block::List {
                ordered: false,
                items: list_items(&self.children, context)?,
            },
            HtmlTag::Ol => Block::List {
                ordered: true,
                items: list_items(&self.children, context)?,
            },
            HtmlTag::Li => {
                return Ok(vec![Block::Paragraph(children_to_inlines(
                    &self.children,
                    context,
                )?)]);
            }
            HtmlTag::Blockquote => {
                Block::Blockquote(flow_children_to_blocks(&self.children, context)?)
            }
            HtmlTag::Table => table(&self.children, context)?,
            HtmlTag::Tr => {
                return Err(HtmlToMarkdownError::UnsupportedTag(self.tag));
            }
            HtmlTag::Th | HtmlTag::Td => {
                return Err(HtmlToMarkdownError::UnsupportedTag(self.tag));
            }
            HtmlTag::A
            | HtmlTag::Strong
            | HtmlTag::Em
            | HtmlTag::Br
            | HtmlTag::Img
            | HtmlTag::Thead
            | HtmlTag::Tbody
            | HtmlTag::Div
            | HtmlTag::Span
            | HtmlTag::Sup
            | HtmlTag::Kbd
            | HtmlTag::Details
            | HtmlTag::Summary => {
                return flow_children_to_blocks(&self.children, context);
            }
        };

        Ok(vec![block])
    }
}

fn nodes_to_blocks(
    nodes: &[HtmlNode],
    context: &MarkdownContext,
) -> Result<Vec<Block>, HtmlToMarkdownError> {
    nodes.iter().try_fold(Vec::new(), |mut blocks, node| {
        blocks.extend(node.to_markdown_blocks_with_context(context)?);
        Ok(blocks)
    })
}

fn flow_children_to_blocks(
    children: &[HtmlNode],
    context: &MarkdownContext,
) -> Result<Vec<Block>, HtmlToMarkdownError> {
    let mut blocks = Vec::new();
    let mut inlines = Vec::new();

    for child in children {
        if let Some(inline) = node_to_inline(child, context)? {
            inlines.push(inline);
            continue;
        }

        push_inline_paragraph(&mut blocks, &mut inlines);
        blocks.extend(child.to_markdown_blocks_with_context(context)?);
    }

    push_inline_paragraph(&mut blocks, &mut inlines);
    Ok(blocks)
}

fn push_inline_paragraph(blocks: &mut Vec<Block>, inlines: &mut Vec<Inline>) {
    if !inlines.is_empty() {
        blocks.push(Block::Paragraph(std::mem::take(inlines)));
    }
}

fn children_to_inlines(
    children: &[HtmlNode],
    context: &MarkdownContext,
) -> Result<Vec<Inline>, HtmlToMarkdownError> {
    children.iter().try_fold(Vec::new(), |mut inlines, child| {
        if let Some(inline) = node_to_inline(child, context)? {
            inlines.push(inline);
        } else {
            inlines.extend(blocks_to_inlines(
                &child.to_markdown_blocks_with_context(context)?,
            ));
        }
        Ok(inlines)
    })
}

fn node_to_inline(
    node: &HtmlNode,
    context: &MarkdownContext,
) -> Result<Option<Inline>, HtmlToMarkdownError> {
    match node {
        HtmlNode::Text(value) => Ok(Some(Inline::Text(value.clone()))),
        HtmlNode::Element(element) => element_to_inline(element, context),
    }
}

fn element_to_inline(
    element: &HtmlElement,
    context: &MarkdownContext,
) -> Result<Option<Inline>, HtmlToMarkdownError> {
    let inline = match element.tag {
        HtmlTag::A => Inline::Link {
            children: children_to_inlines(&element.children, context)?,
            url: resolve_href(context, element.attrs.href.as_deref())?,
            title: element.attrs.title.clone(),
        },
        HtmlTag::Code | HtmlTag::Kbd => Inline::Code(text_content(&element.children)),
        HtmlTag::Strong => Inline::Strong(children_to_inlines(&element.children, context)?),
        HtmlTag::Em => Inline::Emphasis(children_to_inlines(&element.children, context)?),
        HtmlTag::Br => Inline::Break,
        HtmlTag::Img => Inline::Image {
            alt: element.attrs.alt.clone().unwrap_or_default(),
            url: resolve_href(context, element.attrs.src.as_deref())?,
            title: element.attrs.title.clone(),
        },
        HtmlTag::Span | HtmlTag::Sup | HtmlTag::Summary => {
            return Ok(Some(Inline::Text(text_content(&element.children))));
        }
        HtmlTag::H1
        | HtmlTag::H2
        | HtmlTag::H3
        | HtmlTag::H4
        | HtmlTag::H5
        | HtmlTag::H6
        | HtmlTag::P
        | HtmlTag::Pre
        | HtmlTag::Ul
        | HtmlTag::Ol
        | HtmlTag::Li
        | HtmlTag::Blockquote
        | HtmlTag::Table
        | HtmlTag::Thead
        | HtmlTag::Tbody
        | HtmlTag::Tr
        | HtmlTag::Th
        | HtmlTag::Td
        | HtmlTag::Div
        | HtmlTag::Details => return Ok(None),
    };

    Ok(Some(inline))
}

fn resolve_href(
    context: &MarkdownContext,
    href: Option<&str>,
) -> Result<String, HtmlToMarkdownError> {
    let Some(href) = href else {
        return Ok(String::new());
    };

    let Some(source_route) = context.source_route.as_deref() else {
        return Ok(href.to_string());
    };

    resolve_docs_href(source_route, href)
        .map_err(|error| HtmlToMarkdownError::Route(error.to_string()))
}

fn blocks_to_inlines(blocks: &[Block]) -> Vec<Inline> {
    blocks
        .iter()
        .map(|block| Inline::Text(block.to_string()))
        .collect()
}

fn heading(depth: u8, children: Vec<Inline>) -> Block {
    Block::Heading { depth, children }
}

fn code_or_paragraph(value: String) -> Block {
    if value.contains('\n') {
        Block::Code { lang: None, value }
    } else {
        Block::Paragraph(vec![Inline::Code(value)])
    }
}

fn list_items(
    children: &[HtmlNode],
    context: &MarkdownContext,
) -> Result<Vec<ListItem>, HtmlToMarkdownError> {
    children.iter().try_fold(Vec::new(), |mut items, child| {
        match child {
            HtmlNode::Element(element) if element.tag == HtmlTag::Li => {
                items.push(ListItem::new(flow_children_to_blocks(
                    &element.children,
                    context,
                )?));
            }
            HtmlNode::Text(text) if text.trim().is_empty() => {}
            _ => {
                items.push(ListItem::new(
                    child.to_markdown_blocks_with_context(context)?,
                ));
            }
        }
        Ok(items)
    })
}

fn table(children: &[HtmlNode], context: &MarkdownContext) -> Result<Block, HtmlToMarkdownError> {
    let rows = table_rows(children, context)?;
    let column_count = rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or_default();

    Ok(Block::Table(Table {
        rows,
        align: vec![TableAlign::None; column_count],
    }))
}

fn table_rows(
    children: &[HtmlNode],
    context: &MarkdownContext,
) -> Result<Vec<TableRow>, HtmlToMarkdownError> {
    children.iter().try_fold(Vec::new(), |mut rows, child| {
        match child {
            HtmlNode::Element(element) if element.tag == HtmlTag::Tr => {
                rows.push(TableRow::new(table_cells(&element.children, context)?));
            }
            HtmlNode::Element(element)
                if matches!(element.tag, HtmlTag::Thead | HtmlTag::Tbody | HtmlTag::Div) =>
            {
                rows.extend(table_rows(&element.children, context)?);
            }
            HtmlNode::Text(text) if text.trim().is_empty() => {}
            _ => {
                return Err(HtmlToMarkdownError::InvalidTable(format!(
                    "unexpected node under table: {:?}",
                    child
                )));
            }
        }
        Ok(rows)
    })
}

fn table_cells(
    children: &[HtmlNode],
    context: &MarkdownContext,
) -> Result<Vec<TableCell>, HtmlToMarkdownError> {
    children.iter().try_fold(Vec::new(), |mut cells, child| {
        match child {
            HtmlNode::Element(element) if matches!(element.tag, HtmlTag::Th | HtmlTag::Td) => {
                cells.push(TableCell::new(children_to_inlines(
                    &element.children,
                    context,
                )?));
            }
            HtmlNode::Text(text) if text.trim().is_empty() => {}
            _ => {
                cells.push(TableCell::new(
                    child
                        .to_markdown_blocks()?
                        .iter()
                        .map(|block| Inline::Text(block.to_string()))
                        .collect(),
                ));
            }
        }
        Ok(cells)
    })
}

fn text_content(children: &[HtmlNode]) -> String {
    children.iter().map(node_text_content).collect()
}

fn node_text_content(node: &HtmlNode) -> String {
    match node {
        HtmlNode::Text(text) => text.clone(),
        HtmlNode::Element(element) if element.tag == HtmlTag::Br => "\n".to_string(),
        HtmlNode::Element(element) => text_content(&element.children),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs_parser::html::parse_html;
    use crate::docs_parser::md::ToMarkdown;

    #[test]
    fn converts_list_and_table_to_markdown_document() {
        let tree = parse_html(
            "<ul><li>A</li><li>B</li></ul><table><tr><th>K</th></tr><tr><td>V</td></tr></table>",
        )
        .unwrap();
        let root = tree.to_markdown_document().unwrap();

        assert!(matches!(root.children[0], Block::List { .. }));
        assert!(matches!(root.children[1], Block::Table(_)));
        assert_eq!(root.to_markdown(), "- A\n- B\n\n| K |\n| --- |\n| V |");
    }

    #[test]
    fn renders_table_without_external_markdown_renderer() {
        let tree = parse_html(
            r#"<table>
<tr><th>New mode</th><th>Syntax</th><th>Example</th></tr>
<tr><td>Code</td><td>Prefix the code with <code>#</code></td><td><code>Number: #(1+2)</code></td></tr>
</table>"#,
        )
        .unwrap();
        let markdown = tree.to_markdown_document().unwrap().to_markdown();

        assert!(markdown.contains("| New mode | Syntax | Example |"));
        assert!(markdown.contains("| Code | Prefix the code with `#` | `Number: #(1+2)` |"));
    }
}
