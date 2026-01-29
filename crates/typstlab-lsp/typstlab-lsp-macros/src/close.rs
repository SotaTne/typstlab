// typstlab-lsp-macros/src/derive_close.rs
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Fields, Index, parse_macro_input, spanned::Spanned,
};

use crate::close_cfg::{CloseMode, parse_field_cfg, parse_type_default_mode};

/// Represents which method is currently being implemented in the `Close` trait.
#[derive(Clone, Copy)]
enum ImplMethod {
    /// Generating the `close()` method.
    Close,
    /// Generating the `close_and_shrink()` method.
    CloseAndShrink,
}

/// Determines the actual method name to call on a field based on:
/// 1. The method currently being implemented in the trait (`impl_method`).
/// 2. The specific configuration of the field (`field_mode`).
fn pick_call_method(impl_method: ImplMethod, field_mode: CloseMode) -> TokenStream2 {
    match impl_method {
        // If we are implementing `close_and_shrink()`, we always call `close_and_shrink()`
        // on children to ensure full recursive memory release.
        ImplMethod::CloseAndShrink => quote!(close_and_shrink),

        // If we are implementing `close()`, the behavior depends on the child's config.
        ImplMethod::Close => match field_mode {
            // Default: just call `close()`.
            CloseMode::Close => quote!(close),
            // Explicitly requested: call `close_and_shrink()` even during a normal `close()`.
            CloseMode::Shrink => quote!(close_and_shrink),
        },
    }
}

/// Entry point for `#[derive(Close)]`.
pub fn derive_close(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let core_path = crate::utils::core_crate_path();

    let default_mode = match parse_type_default_mode(&input.attrs) {
        Ok(m) => m,
        Err(e) => return e.to_compile_error().into(),
    };

    let body_close = gen_body(&input, default_mode, ImplMethod::Close, &core_path);
    let body_shrink = gen_body(&input, default_mode, ImplMethod::CloseAndShrink, &core_path);

    quote! {
        impl #impl_generics #core_path::Close for #name #ty_generics #where_clause {
            fn close(&mut self) { #body_close }
            fn close_and_shrink(&mut self) { #body_shrink }
        }
    }
    .into()
}

fn gen_body(
    input: &DeriveInput,
    default_mode: CloseMode,
    impl_method: ImplMethod,
    core_path: &TokenStream2,
) -> TokenStream2 {
    match &input.data {
        Data::Struct(ds) => gen_struct(ds, default_mode, impl_method, core_path),
        Data::Enum(de) => gen_enum(de, default_mode, impl_method, core_path),
        Data::Union(u) => {
            let sp = u.union_token.span();
            quote_spanned!(sp=> compile_error!("Close derive does not support unions");)
        }
    }
}

fn gen_struct(
    ds: &DataStruct,
    default_mode: CloseMode,
    impl_method: ImplMethod,
    core_path: &TokenStream2,
) -> TokenStream2 {
    match &ds.fields {
        Fields::Named(fields) => {
            let calls = fields.named.iter().filter_map(|f| {
                let ident = f.ident.as_ref().unwrap();

                let cfg = match parse_field_cfg(&f.attrs, default_mode) {
                    Ok(c) => c,
                    Err(e) => return Some(e.to_compile_error()),
                };
                if cfg.skip {
                    return None;
                }

                let sp = f.span();
                let m = pick_call_method(impl_method, cfg.mode);
                Some(quote_spanned!(sp=> #core_path::Close::#m(&mut self.#ident);))
            });

            quote! { #( #calls )* }
        }

        Fields::Unnamed(fields) => {
            let calls = fields.unnamed.iter().enumerate().filter_map(|(i, f)| {
                let cfg = match parse_field_cfg(&f.attrs, default_mode) {
                    Ok(c) => c,
                    Err(e) => return Some(e.to_compile_error()),
                };
                if cfg.skip {
                    return None;
                }

                let idx = Index::from(i);
                let sp = f.span();
                let m = pick_call_method(impl_method, cfg.mode);
                Some(quote_spanned!(sp=> #core_path::Close::#m(&mut self.#idx);))
            });

            quote! { #( #calls )* }
        }

        Fields::Unit => quote! {},
    }
}

fn gen_enum(
    de: &DataEnum,
    default_mode: CloseMode,
    impl_method: ImplMethod,
    core_path: &TokenStream2,
) -> TokenStream2 {
    let arms = de.variants.iter().map(|v| {
        let v_ident = &v.ident;

        match &v.fields {
            Fields::Unit => quote! { Self::#v_ident => {} },

            Fields::Unnamed(fields) => {
                // パターンの各要素を、skipなら `_`、それ以外なら `ref mut __f{i}` にする
                let mut pat_elems = Vec::<TokenStream2>::new();
                let mut calls = Vec::<TokenStream2>::new();

                for (i, f) in fields.unnamed.iter().enumerate() {
                    let cfg = match parse_field_cfg(&f.attrs, default_mode) {
                        Ok(c) => c,
                        Err(e) => return e.to_compile_error(),
                    };

                    let sp = f.span();

                    if cfg.skip {
                        pat_elems.push(quote_spanned!(sp=> _));
                        continue;
                    }

                    let b = format_ident!("__f{i}");
                    pat_elems.push(quote_spanned!(sp=> #b));

                    let m = pick_call_method(impl_method, cfg.mode);
                    calls.push(quote_spanned!(sp=> #core_path::Close::#m(#b);));
                }

                quote! {
                    Self::#v_ident( #( #pat_elems ),* ) => { #( #calls )* }
                }
            }

            Fields::Named(fields) => {
                // named は「必要なものだけ bind して、残りは `..`」が一番安定
                let mut pat_elems = Vec::<TokenStream2>::new();
                let mut calls = Vec::<TokenStream2>::new();

                for f in fields.named.iter() {
                    let ident = f.ident.clone().unwrap();
                    let cfg = match parse_field_cfg(&f.attrs, default_mode) {
                        Ok(c) => c,
                        Err(e) => return e.to_compile_error(),
                    };
                    let sp = f.span();

                    if cfg.skip {
                        continue;
                    }

                    // `x` (auto-binds to `ref mut x` in 2024)
                    pat_elems.push(quote_spanned!(sp=> #ident));

                    let m = pick_call_method(impl_method, cfg.mode);
                    calls.push(quote_spanned!(sp=> #core_path::Close::#m(#ident);));
                }

                quote! {
                    Self::#v_ident { #( #pat_elems, )* .. } => { #( #calls )* }
                }
            }
        }
    });

    quote! {
        match self {
            #( #arms ),*
        }
    }
}
