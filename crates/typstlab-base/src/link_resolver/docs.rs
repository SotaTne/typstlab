use super::ResolvedLink;
use crate::version_resolver::ResolvedVersionSet;
use typstlab_proto::SourceFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsLinkRequest {
    pub versions: ResolvedVersionSet,
}

pub fn resolve_docs_link(request: DocsLinkRequest) -> ResolvedLink {
    ResolvedLink {
        url: format!(
            "https://github.com/typst-community/dev-builds/releases/download/docs-v{}/docs.json",
            request.versions.docs_version
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
            versions: ResolvedVersionSet {
                typst_version: "0.14.2".to_string(),
                docs_version: "0.14.2".to_string(),
            },
        });

        assert_eq!(
            link.url,
            "https://github.com/typst-community/dev-builds/releases/download/docs-v0.14.2/docs.json"
        );
        assert_eq!(link.format, SourceFormat::Raw);
    }
}
