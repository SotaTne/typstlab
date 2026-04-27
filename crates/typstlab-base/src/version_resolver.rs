#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVersionSet {
    pub typst_version: String,
    pub docs_version: String,
}

pub fn resolve_versions_from_typst(typst_version: &str) -> ResolvedVersionSet {
    let typst_version = typst_version.strip_prefix('v').unwrap_or(typst_version);

    ResolvedVersionSet {
        typst_version: typst_version.to_string(),
        docs_version: typst_version.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_versions_from_typst_maps_docs_one_to_one() {
        let versions = resolve_versions_from_typst("0.14.2");

        assert_eq!(versions.typst_version, "0.14.2");
        assert_eq!(versions.docs_version, "0.14.2");
    }

    #[test]
    fn test_resolve_versions_from_typst_normalizes_leading_v() {
        let versions = resolve_versions_from_typst("v0.14.2");

        assert_eq!(versions.typst_version, "0.14.2");
        assert_eq!(versions.docs_version, "0.14.2");
    }
}
