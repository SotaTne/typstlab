use crate::utils::core_crate_path;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, parse_macro_input};

use crate::feature_cfg::parse_feature_cfg;

pub fn derive_feature(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // union は拒否（Close と同じ方針）
    if matches!(input.data, Data::Union(_)) {
        return Error::new_spanned(&input, "Feature derive does not support unions")
            .to_compile_error()
            .into();
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let core_path = core_crate_path();

    let cfg = match parse_feature_cfg(&input.attrs, &core_path) {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };

    let ignore = cfg.ignore;

    // ignore のときは空
    let features = if ignore {
        quote! { &[] }
    } else {
        let items = cfg.items;
        // items が空でも OK（=「この型自身は feature を持たない」が明示できる）
        // ただし “忘れ” が怖いなら、ここで empty をエラーにするのもアリ。
        quote! { &[ #( #items ),* ] }
    };

    let expanded = quote! {
        impl #impl_generics #core_path::Feature for #name #ty_generics #where_clause {
            const IGNORE: bool = #ignore;
            const FEATURES: &'static [#core_path::FeatureId] = #features;
        }
    };

    expanded.into()
}
