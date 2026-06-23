use super::ast::{Block, Document, Inline, ListItem, Table, TableAlign};

pub fn document_to_markdown(document: &Document) -> String {
    render_blocks(&document.children, 0)
}

pub fn block_to_markdown(block: &Block) -> String {
    render_block(block, 0)
}

pub fn inline_to_markdown(inline: &Inline) -> String {
    render_inline(inline, false)
}

fn render_blocks(blocks: &[Block], indent: usize) -> String {
    blocks
        .iter()
        .filter_map(|block| {
            let markdown = render_block(block, indent);
            if markdown.is_empty() {
                None
            } else {
                Some(markdown)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_block(block: &Block, indent: usize) -> String {
    match block {
        Block::Paragraph(children) => prefix_lines(&render_inlines(children, false), indent),
        Block::Heading { depth, children } => {
            let depth = (*depth).clamp(1, 6) as usize;
            format!(
                "{}{} {}",
                " ".repeat(indent),
                "#".repeat(depth),
                render_inlines(children, false)
            )
        }
        Block::Code { lang, value } => render_code_block(lang.as_deref(), value, indent),
        Block::Blockquote(children) => render_blockquote(children, indent),
        Block::List { ordered, items } => render_list(*ordered, items, indent),
        Block::Table(table) => render_table(table, indent),
        Block::ThematicBreak => format!("{}---", " ".repeat(indent)),
    }
}

fn render_code_block(lang: Option<&str>, value: &str, indent: usize) -> String {
    let fence = code_fence(value);
    let prefix = " ".repeat(indent);
    let lang = lang.unwrap_or_default();
    let value = value.trim_matches('\n');

    if value.is_empty() {
        return format!("{prefix}{fence}{lang}\n{prefix}{fence}");
    }

    let body = value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{prefix}{fence}{lang}\n{body}\n{prefix}{fence}")
}

fn code_fence(value: &str) -> String {
    let longest = longest_backtick_run(value);
    "`".repeat(3.max(longest + 1))
}

fn render_blockquote(children: &[Block], indent: usize) -> String {
    let rendered = render_blocks(children, 0);
    if rendered.is_empty() {
        return String::new();
    }

    let prefix = format!("{}> ", " ".repeat(indent));
    rendered
        .lines()
        .map(|line| {
            if line.is_empty() {
                format!("{}>", " ".repeat(indent))
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_list(ordered: bool, items: &[ListItem], indent: usize) -> String {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| render_list_item(ordered, index + 1, item, indent))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_list_item(ordered: bool, number: usize, item: &ListItem, indent: usize) -> String {
    let marker = if ordered {
        format!("{number}. ")
    } else {
        "- ".to_string()
    };
    let prefix = " ".repeat(indent);
    let child_indent = indent + marker.len();

    let Some((first, rest)) = item.children.split_first() else {
        return format!("{prefix}{}", marker.trim_end());
    };

    let mut lines = Vec::new();
    match first {
        Block::Paragraph(children) => {
            lines.push(format!(
                "{prefix}{marker}{}",
                render_inlines(children, false)
            ));
        }
        block => {
            lines.push(format!("{prefix}{}", marker.trim_end()));
            lines.push(render_block(block, child_indent));
        }
    }

    for block in rest {
        lines.push(render_block(block, child_indent));
    }

    lines.join("\n")
}

fn render_table(table: &Table, indent: usize) -> String {
    if table.rows.is_empty() {
        return String::new();
    }

    let column_count = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or_default();
    if column_count == 0 {
        return String::new();
    }

    let prefix = " ".repeat(indent);
    let mut lines = Vec::new();
    lines.push(format!(
        "{prefix}| {} |",
        render_table_cells(&table.rows[0].cells, column_count)
    ));
    lines.push(format!(
        "{prefix}| {} |",
        (0..column_count)
            .map(|index| render_table_separator(
                table.align.get(index).copied().unwrap_or_default()
            ))
            .collect::<Vec<_>>()
            .join(" | ")
    ));

    for row in table.rows.iter().skip(1) {
        lines.push(format!(
            "{prefix}| {} |",
            render_table_cells(&row.cells, column_count)
        ));
    }

    lines.join("\n")
}

fn render_table_separator(align: TableAlign) -> &'static str {
    match align {
        TableAlign::None => "---",
        TableAlign::Left => ":---",
        TableAlign::Right => "---:",
        TableAlign::Center => ":---:",
    }
}

fn render_table_cells(cells: &[super::ast::TableCell], column_count: usize) -> String {
    (0..column_count)
        .map(|index| {
            cells
                .get(index)
                .map(|cell| escape_table_cell(&render_inlines(&cell.children, true)))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_inlines(inlines: &[Inline], in_table_cell: bool) -> String {
    inlines
        .iter()
        .map(|inline| render_inline(inline, in_table_cell))
        .collect::<String>()
}

fn render_inline(inline: &Inline, in_table_cell: bool) -> String {
    match inline {
        Inline::Text(value) => {
            let _ = in_table_cell;
            value.clone()
        }
        Inline::Code(value) => render_inline_code(value),
        Inline::Emphasis(children) => format!("*{}*", render_inlines(children, in_table_cell)),
        Inline::Strong(children) => format!("**{}**", render_inlines(children, in_table_cell)),
        Inline::Link {
            children,
            url,
            title,
        } => {
            let title = title
                .as_ref()
                .map(|title| format!(" \"{}\"", title.replace('"', "\\\"")))
                .unwrap_or_default();
            format!(
                "[{}]({}{})",
                render_inlines(children, in_table_cell),
                url,
                title
            )
        }
        Inline::Image { alt, url, title } => {
            let title = title
                .as_ref()
                .map(|title| format!(" \"{}\"", title.replace('"', "\\\"")))
                .unwrap_or_default();
            format!("![{}]({}{})", alt, url, title)
        }
        Inline::Break => "  \n".to_string(),
    }
}

fn render_inline_code(value: &str) -> String {
    let ticks = "`".repeat(longest_backtick_run(value) + 1);
    format!("{ticks}{value}{ticks}")
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

fn longest_backtick_run(value: &str) -> usize {
    value
        .chars()
        .fold((0usize, 0usize), |(current, max), character| {
            if character == '`' {
                let current = current + 1;
                (current, max.max(current))
            } else {
                (0, max)
            }
        })
        .1
}

fn prefix_lines(value: &str, indent: usize) -> String {
    if indent == 0 {
        return value.to_string();
    }

    let prefix = " ".repeat(indent);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::ast::{
        Block, Document, Inline, ListItem, Table, TableAlign, TableCell, TableRow, ToMarkdown,
    };

    #[test]
    fn renders_document_to_string() {
        let document = Document::new(vec![
            Block::Heading {
                depth: 1,
                children: vec![Inline::text("Title")],
            },
            Block::Paragraph(vec![
                Inline::text("Hello "),
                Inline::Strong(vec![Inline::text("world")]),
                Inline::text("."),
            ]),
        ]);

        assert_eq!(document.to_string(), "# Title\n\nHello **world**.");
        let markdown: String = document.into();
        assert_eq!(markdown, "# Title\n\nHello **world**.");
    }

    #[test]
    fn renders_code_block_with_safe_fence() {
        let block = Block::Code {
            lang: Some("typ".to_string()),
            value: "```typ\n#let x = 1\n```".to_string(),
        };

        assert_eq!(
            block.to_markdown(),
            "````typ\n```typ\n#let x = 1\n```\n````"
        );
    }

    #[test]
    fn renders_gfm_table() {
        let table = Block::Table(Table {
            align: vec![TableAlign::Left, TableAlign::Center],
            rows: vec![
                TableRow::new(vec![
                    TableCell::new(vec![Inline::text("Name")]),
                    TableCell::new(vec![Inline::text("Syntax")]),
                ]),
                TableRow::new(vec![
                    TableCell::new(vec![Inline::text("Code")]),
                    TableCell::new(vec![Inline::code("Number: #(1|2)")]),
                ]),
            ],
        });

        assert_eq!(
            table.to_markdown(),
            "| Name | Syntax |\n| :--- | :---: |\n| Code | `Number: #(1\\|2)` |"
        );
    }

    #[test]
    fn renders_nested_blocks() {
        let document = Document::new(vec![
            Block::List {
                ordered: false,
                items: vec![ListItem::new(vec![
                    Block::Paragraph(vec![Inline::text("Item")]),
                    Block::List {
                        ordered: true,
                        items: vec![ListItem::new(vec![Block::Paragraph(vec![Inline::text(
                            "Nested",
                        )])])],
                    },
                ])],
            },
            Block::Blockquote(vec![Block::Paragraph(vec![Inline::text("Quote")])]),
        ]);

        assert_eq!(document.to_markdown(), "- Item\n  1. Nested\n\n> Quote");
    }
}
