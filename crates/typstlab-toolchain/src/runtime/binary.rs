use crate::{StoredBinary, TypedBinaryTool};

#[derive(Debug, Clone)]
pub struct TypedResolvedBinary<'a, T>
where
    T: TypedBinaryTool,
{
    pub tool: &'a T,
    pub commands: &'a T::Commands,
    pub stored: StoredBinary,
    pub version: String,
}

impl<'a, T> TypedResolvedBinary<'a, T>
where
    T: TypedBinaryTool,
{
    pub fn new(tool: &'a T, stored: StoredBinary, version: impl Into<String>) -> Self {
        Self {
            tool,
            commands: tool.commands(),
            stored,
            version: version.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::binary::typst;
    use crate::{BinaryTool, StoredBinary, TypedResolvedBinary};

    #[test]
    fn typed_resolved_binary_uses_tool_commands() {
        let stored = StoredBinary {
            root: "store/typst".into(),
        };
        let resolved = TypedResolvedBinary::new(&typst::TOOL, stored, "0.14.2");

        assert_eq!(resolved.tool.id(), "typst");
        assert_eq!(resolved.version, "0.14.2");
        assert!(std::ptr::eq(resolved.commands, &typst::COMMAND));
    }
}
