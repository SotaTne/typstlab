use crate::sclap_context_cfg::{SclapContextCfg, parse_struct_fields};
use crate::utils::core_crate_path;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error, parse_macro_input};

pub fn derive_sclap_context(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let core_path = core_crate_path();

    match derive_sclap_context_impl(input, core_path) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

pub(crate) fn derive_sclap_context_impl(
    input: DeriveInput,
    core_path: TokenStream2,
) -> syn::Result<TokenStream2> {
    // union,enumはサポートしない
    if matches!(input.data, Data::Enum(_) | Data::Union(_)) {
        return Err(Error::new_spanned(
            input.ident,
            "SclapContext can only be derived for structs",
        ));
    }

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return Err(Error::new_spanned(
                &input.ident,
                "SclapContext can only be derived for structs",
            ));
        }
    };
    let struct_name = input.ident;
    let cfg = parse_struct_fields(fields)?;
    Ok(generate_sclap_context_impl(
        &struct_name,
        &input.generics,
        &cfg,
        core_path,
    ))
}

pub(crate) fn generate_sclap_context_impl(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    cfg: &SclapContextCfg,
    core_path: TokenStream2,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // 1. 展開用のデータを「変数」として外側に切り出す
    // keys を Vec<&String> のイテレータとして準備
    let keys_iter = cfg.keys.iter();

    // 2. match_arms も同様に Vec 等に集約しておくのが最も安全
    let match_arms: Vec<_> = keys_iter
        .clone()
        .map(|key| {
            let (ident, validator_path) = &cfg.validator_map[key];
            quote! {
                #key => #validator_path(&self.#ident, req_val),
            }
        })
        .collect();

    // 3. 最終的なコード生成
    quote! {
        impl #impl_generics #core_path::SclapContext for #struct_name #ty_generics #where_clause {
            // 変数をそのまま渡す。これで CheckHasIterator はパスする。
            const KEYS: &'static [&'static str] = &[ #(#keys_iter),* ];

            fn check_all(&self, requirements: &[(&str, &str)]) -> bool {
                requirements.iter().all(|(req_key, req_val)| {
                    match *req_key {
                        #(#match_arms)*
                        _ => false,
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use syn::parse_quote;

    fn normalized(ts: &TokenStream2) -> String {
        ts.to_string()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect()
    }

    fn sample_cfg() -> SclapContextCfg {
        let mut validator_map = HashMap::new();
        validator_map.insert(
            "version".to_string(),
            (
                parse_quote!(version),
                parse_quote!(validators::version_check),
            ),
        );

        SclapContextCfg {
            keys: vec!["version".to_string()],
            validator_map,
        }
    }

    // derive_sclap_context_impl: input validation + cfg parsing orchestration.
    mod derive_sclap_context_impl_tests {
        use super::*;

        #[test]
        fn errors_for_enum_input() {
            let input: DeriveInput = parse_quote! {
                enum Demo { A }
            };
            let err =
                derive_sclap_context_impl(input, quote!(::sclap_core)).expect_err("should fail");
            assert!(err.to_string().contains("only be derived for structs"));
        }

        #[test]
        fn propagates_cfg_parse_error() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    version: String,
                }
            };
            let err =
                derive_sclap_context_impl(input, quote!(::sclap_core)).expect_err("should fail");
            assert!(err.to_string().contains("requires `#[sclap(...)]"));
        }

        #[test]
        fn generates_impl_for_valid_input() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(key = "version", validator = validators::version_check)]
                    version: String,
                }
            };
            let tokens =
                derive_sclap_context_impl(input, quote!(::sclap_core)).expect("should parse");
            let got = normalized(&tokens);

            assert!(got.contains("impl::sclap_core::SclapContextforDemo"));
            assert!(got.contains("constKEYS:&'static[&'staticstr]=&[\"version\"]"));
            assert!(got.contains("\"version\"=>validators::version_check(&self.version,req_val),"));
        }
    }

    // generate_sclap_context_impl: token generation from parsed cfg.
    mod generate_sclap_context_impl_tests {
        use super::*;

        #[test]
        fn emits_keys_and_match_arms() {
            let struct_name = parse_quote!(MyContext);
            let generics: syn::Generics = parse_quote!();
            let cfg = sample_cfg();

            let tokens =
                generate_sclap_context_impl(&struct_name, &generics, &cfg, quote!(::sclap_core));
            let got = normalized(&tokens);

            assert!(got.contains("impl::sclap_core::SclapContextforMyContext"));
            assert!(got.contains("constKEYS:&'static[&'staticstr]=&[\"version\"]"));
            assert!(got.contains("match*req_key"));
            assert!(got.contains("\"version\"=>validators::version_check(&self.version,req_val),"));
            assert!(got.contains("_=>false"));
        }

        #[test]
        fn preserves_generics_and_where_clause() {
            let struct_name = parse_quote!(MyContext);
            let input: DeriveInput = parse_quote! {
                struct Dummy<T> where T: Clone {
                    value: T,
                }
            };
            let generics = input.generics;
            let cfg = sample_cfg();

            let tokens =
                generate_sclap_context_impl(&struct_name, &generics, &cfg, quote!(::sclap_core));
            let got = normalized(&tokens);

            assert!(got.contains("impl<T>::sclap_core::SclapContextforMyContext<T>whereT:Clone"));
        }
    }
}
