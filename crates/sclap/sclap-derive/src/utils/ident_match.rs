macro_rules! ident_match {
    ($attr:expr, $head:literal $(| $tail:literal)* $(|)?) => {{
        let attr_ref: &syn::Attribute = &$attr;
        let path = attr_ref.path();
        path.is_ident($head) $(|| path.is_ident($tail))*
    }};
}

pub(crate) use ident_match;

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    #[test]
    fn matches_single_ident() {
        let attr: syn::Attribute = parse_quote!(#[feat]);
        assert!(super::ident_match!(attr, "feat"));
    }

    #[test]
    fn matches_one_of_multiple_idents() {
        let attr: syn::Attribute = parse_quote!(#[guard]);
        assert!(super::ident_match!(attr, "feat" | "guard"));
    }

    #[test]
    fn returns_false_when_no_idents_match() {
        let attr: syn::Attribute = parse_quote!(#[other]);
        assert!(!super::ident_match!(attr, "feat" | "guard"));
    }
}
