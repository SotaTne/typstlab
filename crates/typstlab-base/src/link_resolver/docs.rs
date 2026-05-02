use super::{ResolvedLink, Version};
use typstlab_proto::SourceFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsLinkRequest<'a> {
    pub version: Version<'a>,
}

pub fn resolve_docs_link(request: DocsLinkRequest<'_>) -> ResolvedLink {
    ResolvedLink {
        url: format!(
            "https://github.com/typst-community/dev-builds/releases/download/docs-v{}/docs.json",
            request.version.as_str()
        ),
        format: SourceFormat::Raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_docs_link_uses_docs_version_and_raw_format() {
        let link = resolve_docs_link(DocsLinkRequest {
            version: Version::new("0.14.2"),
        });

        assert_eq!(
            link.url,
            "https://github.com/typst-community/dev-builds/releases/download/docs-v0.14.2/docs.json"
        );
        assert_eq!(link.format, SourceFormat::Raw);
    }
}
