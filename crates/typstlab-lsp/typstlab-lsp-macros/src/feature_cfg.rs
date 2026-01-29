use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;
use syn::{Attribute, Error, Result, spanned::Spanned};

/// Configuration for the `Feature` derive macro.
#[derive(Debug)]
pub struct FeatureCfg {
    /// If true (`#[feature(ignore)]`), this type is excluded from feature checks.
    pub ignore: bool,
    /// Token streams representing `FeatureId` values (e.g., `::typstlab_lsp_core::FeatureId::X`).
    pub items: Vec<TokenStream2>,
}

/// Parses the `#[feat(...)]` attribute.
pub fn parse_feature_cfg(attrs: &[Attribute], core_path: &TokenStream2) -> Result<FeatureCfg> {
    let mut found_attr = None;

    for a in attrs {
        if !a.path().is_ident("feat") {
            continue;
        }
        if found_attr.is_some() {
            return Err(Error::new_spanned(
                a,
                "multiple `#[feat(...)]` attributes are not allowed",
            ));
        }
        found_attr = Some(a);
    }

    let Some(attr) = found_attr else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "`#[derive(Feature)]` requires `#[feat(...)]`. \
Use `#[feat(ignore)]` if you want to exclude this type from feature checking.",
        ));
    };

    let mut ignore = false;
    let mut items: Vec<TokenStream2> = vec![];

    attr.parse_nested_meta(|meta| {
        // `#[feat(ignore)]`
        if meta.path.is_ident("ignore") {
            ignore = true;
            return Ok(());
        }

        // Treat any other path as a FeatureId.
        // It normalizes `SugarFoo` or `FeatureId::SugarFoo` to `::core_path::FeatureId::SugarFoo`.
        let path = &meta.path;

        // Parentheses or equality (e.g., `feat(x=...)`) are not allowed.
        if meta.input.peek(syn::token::Paren) || meta.input.peek(syn::token::Eq) {
            return Err(Error::new_spanned(
                meta.path,
                "invalid `#[feat(...)]` syntax; use paths only, e.g. `#[feat(SugarFoo)]`",
            ));
        }

        let segs: Vec<_> = path.segments.iter().collect();
        let last = segs.last().unwrap();
        let last_ident = &last.ident;

        // Use only the last segment for normalization (e.g., `FeatureId::X` -> `::core_path::FeatureId::X`)
        let ts = quote_spanned!(path.span()=> #core_path::FeatureId::#last_ident);
        items.push(ts);

        Ok(())
    })?;

    // Rule: `ignore` and feature items cannot be combined.
    if ignore && !items.is_empty() {
        return Err(Error::new_spanned(
            attr,
            "`#[feat(ignore)]` cannot be combined with feature items",
        ));
    }

    Ok(FeatureCfg { ignore, items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::DeriveInput;

    fn parse_attrs(tokens: TokenStream2) -> Vec<Attribute> {
        let input: DeriveInput = syn::parse2(quote! {
            #tokens
            struct Dummy;
        })
        .unwrap();
        input.attrs
    }

    fn core_path() -> TokenStream2 {
        quote!(::typstlab_lsp_core)
    }

    #[test]
    fn test_parse_ignore() {
        let attrs = parse_attrs(quote! { #[feat(ignore)] });
        let cfg = parse_feature_cfg(&attrs, &core_path()).unwrap();
        assert!(cfg.ignore);
        assert!(cfg.items.is_empty());
    }

    #[test]
    fn test_parse_items() {
        let attrs = parse_attrs(quote! { #[feat(SugarFoo, FeatureId::Bar)] });
        let cfg = parse_feature_cfg(&attrs, &core_path()).unwrap();
        assert!(!cfg.ignore);
        assert_eq!(cfg.items.len(), 2);
        // Normalized strings check
        let s0 = cfg.items[0].to_string();
        let s1 = cfg.items[1].to_string();
        assert!(s0.contains("FeatureId :: SugarFoo"));
        assert!(s1.contains("FeatureId :: Bar"));
    }

    #[test]
    fn test_conflict_ignore_items() {
        let attrs = parse_attrs(quote! { #[feat(ignore, SugarFoo)] });
        let res = parse_feature_cfg(&attrs, &core_path());
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("cannot be combined"));
    }

    #[test]
    fn test_missing_attr() {
        let attrs = parse_attrs(quote! {});
        let res = parse_feature_cfg(&attrs, &core_path());
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("requires `#[feat(...)]`")
        );
    }

    #[test]
    fn test_multiple_attrs() {
        let attrs = parse_attrs(quote! {
            #[feat(ignore)]
            #[feat(SugarFoo)]
        });
        let res = parse_feature_cfg(&attrs, &core_path());
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("multiple `#[feat(...)]` attributes")
        );
    }
}
