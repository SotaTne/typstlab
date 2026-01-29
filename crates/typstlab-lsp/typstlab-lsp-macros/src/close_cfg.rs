// typstlab-lsp-macros/src/close_cfg.rs
use syn::{Attribute, Error, Meta, Result};

/// Determines the behavior when `Close::close()` is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseMode {
    /// Normal logical cleanup (calls `.close()`).
    Close,
    /// Cleanup with physical memory release (calls `.close_and_shrink()`).
    Shrink,
}

/// Raw flags parsed from a `#[close(...)]` attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloseFlags {
    /// If true (`#[close(skip)]`), this field/type is ignored.
    pub skip: bool,
    /// If true (`#[close(shrink)]`), it forces memory release.
    pub shrink: bool,
}

/// Finalized configuration for a specific field.
#[derive(Clone, Copy, Debug)]
pub struct FieldCfg {
    /// Whether to skip this field.
    pub skip: bool,
    /// The mode (Close or Shrink) to use when the parent is being "closed".
    pub mode: CloseMode,
}

impl FieldCfg {
    pub const fn new(skip: bool, mode: CloseMode) -> Self {
        Self { skip, mode }
    }
}

fn ensure_list_attr(a: &Attribute) -> Result<()> {
    match &a.meta {
        Meta::List(_) => Ok(()),
        _ => Err(Error::new_spanned(
            a,
            "expected `#[close(...)]` (e.g. `#[close(shrink)]` / `#[close(skip)]`)",
        )),
    }
}

/// attrs から `#[close(...)]` を全部読み、フラグ集合に正規化する
pub fn parse_close_flags(attrs: &[Attribute]) -> Result<CloseFlags> {
    let mut flags = CloseFlags::default();

    for a in attrs {
        if !a.path().is_ident("close") {
            continue;
        }
        ensure_list_attr(a)?;

        a.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                flags.skip = true;
                return Ok(());
            }
            if meta.path.is_ident("shrink") {
                flags.shrink = true;
                return Ok(());
            }
            Err(Error::new_spanned(
                meta.path,
                "unknown `#[close(...)]` option (allowed: skip, shrink)",
            ))
        })?;
    }

    Ok(flags)
}

/// Type-level default:
/// - `#[close(shrink)]` => default_mode = Shrink
/// - otherwise Close
///
/// Disallow:
/// - `#[close(skip)]` on type
pub fn parse_type_default_mode(attrs: &[Attribute]) -> Result<CloseMode> {
    let flags = parse_close_flags(attrs)?;

    if flags.skip {
        // `skip` の位置をちゃんと指したいなら、close属性を探してそこを spanned にしてもOK
        return Err(Error::new_spanned(
            attrs.iter().find(|a| a.path().is_ident("close")).unwrap(),
            "`#[close(skip)]` is not allowed on types (use it on fields)",
        ));
    }

    Ok(if flags.shrink {
        CloseMode::Shrink
    } else {
        CloseMode::Close
    })
}

/// Field-level config:
/// - `#[close(skip)]` => skip = true
/// - `#[close(shrink)]` => mode = Shrink
/// - none => mode = default_mode
pub fn parse_field_cfg(attrs: &[Attribute], default_mode: CloseMode) -> Result<FieldCfg> {
    let flags = parse_close_flags(attrs)?;

    if flags.skip && flags.shrink {
        return Err(Error::new_spanned(
            attrs.iter().find(|a| a.path().is_ident("close")).unwrap(),
            "`#[close(skip)]` and `#[close(shrink)]` cannot be combined on the same field",
        ));
    }

    Ok(FieldCfg::new(
        flags.skip,
        if flags.shrink {
            CloseMode::Shrink
        } else {
            default_mode
        },
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{Attribute, parse_quote};

    fn parse_attrs(tokens: proc_macro2::TokenStream) -> Vec<Attribute> {
        let attrs: syn::DeriveInput = parse_quote!( #tokens struct Dummy; );
        attrs.attrs
    }

    #[test]
    fn test_parse_close_flags() {
        let attrs = parse_attrs(quote! {
            #[close(skip)]
            #[close(shrink)]
        });
        let flags = parse_close_flags(&attrs).unwrap();
        assert!(flags.skip);
        assert!(flags.shrink);

        let attrs = parse_attrs(quote! { #[other] });
        let flags = parse_close_flags(&attrs).unwrap();
        assert!(!flags.skip);
        assert!(!flags.shrink);
    }

    #[test]
    fn test_parse_type_default_mode() {
        let attrs = parse_attrs(quote! { #[close(shrink)] });
        assert_eq!(parse_type_default_mode(&attrs).unwrap(), CloseMode::Shrink);

        let attrs = parse_attrs(quote! {});
        assert_eq!(parse_type_default_mode(&attrs).unwrap(), CloseMode::Close);

        let attrs = parse_attrs(quote! { #[close(skip)] });
        assert!(parse_type_default_mode(&attrs).is_err());
    }

    #[test]
    fn test_parse_field_cfg() {
        // default_mode = Close
        let attrs = parse_attrs(quote! { #[close(shrink)] });
        let cfg = parse_field_cfg(&attrs, CloseMode::Close).unwrap();
        assert!(!cfg.skip);
        assert_eq!(cfg.mode, CloseMode::Shrink);

        // default_mode = Shrink, no attr
        let attrs = parse_attrs(quote! {});
        let cfg = parse_field_cfg(&attrs, CloseMode::Shrink).unwrap();
        assert!(!cfg.skip);
        assert_eq!(cfg.mode, CloseMode::Shrink);

        // skip
        let attrs = parse_attrs(quote! { #[close(skip)] });
        let cfg = parse_field_cfg(&attrs, CloseMode::Close).unwrap();
        assert!(cfg.skip);
    }

    #[test]
    fn test_precedence() {
        // skip on field with Shrink default mode -> skip wins
        let attrs = parse_attrs(quote! { #[close(skip)] });
        let cfg = parse_field_cfg(&attrs, CloseMode::Shrink).unwrap();
        assert!(cfg.skip);

        // shrink on field with Close default mode -> shrink wins
        let attrs = parse_attrs(quote! { #[close(shrink)] });
        let cfg = parse_field_cfg(&attrs, CloseMode::Close).unwrap();
        assert_eq!(cfg.mode, CloseMode::Shrink);
    }

    #[test]
    fn test_conflict_skip_shrink() {
        // Case 1: Combined in a single attribute list
        let attrs = parse_attrs(quote! {
            #[close(skip, shrink)]
        });
        let res = parse_field_cfg(&attrs, CloseMode::Close);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("cannot be combined"));

        // Case 2: Specified across multiple attributes
        let attrs = parse_attrs(quote! {
            #[close(skip)]
            #[close(shrink)]
        });
        let res = parse_field_cfg(&attrs, CloseMode::Close);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("cannot be combined"));
    }
}
