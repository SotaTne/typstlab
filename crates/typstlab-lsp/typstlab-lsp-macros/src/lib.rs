use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// Placeholder for a derive macro
#[proc_macro_derive(MyTrait)]
pub fn my_trait_derive(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    let expanded = quote! {};
    TokenStream::from(expanded)
}
