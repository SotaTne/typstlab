use crate::utils::ident_match;
use std::collections::HashMap;
use syn::{Error, Field, Fields, Ident, Path, Result};

pub(crate) struct SclapContextCfg {
    pub(crate) keys: Vec<String>,
    pub(crate) validator_map: HashMap<String, (Ident, Path)>,
}

fn parse_field_sclap_attr(field: &Field) -> Result<(String, (Ident, Path))> {
    let mut found_attr = None;

    for attr in &field.attrs {
        if !ident_match!(attr, "sclap") {
            continue;
        }
        if found_attr.is_some() {
            return Err(Error::new_spanned(
                attr,
                "multiple `#[sclap(...)]` attributes are not allowed on the same field",
            ));
        }
        found_attr = Some(attr);
    }

    let Some(attr) = found_attr else {
        return Err(Error::new_spanned(
            field,
            "#[derive(SclapContext)]` requires `#[sclap(...)].",
        ));
    };

    let ident = field.ident.clone().ok_or_else(|| {
        Error::new_spanned(field, "Tuple structs are not supported by SclapContext")
    })?;

    let mut key = ident.to_string();
    let mut some_validator_path: Option<Path> = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("key") {
            // もし、keyが指定されていなかったらidentがkeyになる
            let s: syn::LitStr = meta.value()?.parse()?;
            key = s.value();
            Ok(())
        } else if meta.path.is_ident("validator") {
            // パス (関数名) として直接パース
            let path: Path = meta.value()?.parse()?;
            some_validator_path = Some(path);
            Ok(())
        } else {
            Err(meta.error("unsupported sclap attribute property"))
        }
    })?;

    let validator_path = some_validator_path.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "Missing `validator` attribute. Example: #[sclap(validator = my_validator_fn)]",
        )
    })?;

    Ok((key, (ident, validator_path)))
}

pub(crate) fn parse_struct_fields(fields: &Fields) -> Result<SclapContextCfg> {
    let mut keys = Vec::new();
    let mut validator_map = HashMap::new();

    for field in fields {
        let (key, (ident, validator_path)) = parse_field_sclap_attr(field)?;
        if keys.contains(&key) {
            return Err(Error::new_spanned(
                field,
                format!("Duplicate key `{key}` found in SclapContext fields"),
            ));
        }
        keys.push(key.clone());
        validator_map.insert(key, (ident, validator_path));
    }

    Ok(SclapContextCfg {
        keys,
        validator_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{Data, DeriveInput, Path, parse_quote};

    fn fields_of(input: DeriveInput) -> Fields {
        match input.data {
            Data::Struct(data) => data.fields,
            _ => panic!("expected struct input"),
        }
    }

    fn parse_named_field(src: proc_macro2::TokenStream) -> Field {
        parse_quote!(#src)
    }

    // parse_field_sclap_attr: single field parser behavior.
    mod parse_field_sclap_attr_tests {
        use super::*;

        #[test]
        fn parses_key_and_validator() {
            let field = parse_named_field(quote::quote!(
                #[sclap(key = "user.name", validator = validators::name)]
                name: String
            ));

            let (key, (ident, path)) = parse_field_sclap_attr(&field).expect("should parse");
            let expected_path: Path = parse_quote!(validators::name);

            assert_eq!(key, "user.name");
            assert_eq!(ident.to_string(), "name");
            assert_eq!(path, expected_path);
        }

        #[test]
        fn uses_ident_as_default_key() {
            let field = parse_named_field(quote::quote!(
                #[sclap(validator = validators::mode)]
                mode: String
            ));

            let (key, (ident, path)) = parse_field_sclap_attr(&field).expect("should parse");
            let expected_path: Path = parse_quote!(validators::mode);

            assert_eq!(key, "mode");
            assert_eq!(ident.to_string(), "mode");
            assert_eq!(path, expected_path);
        }

        #[test]
        fn errors_when_sclap_attr_is_missing() {
            let field = parse_named_field(quote::quote!(
                name: String
            ));

            let err = match parse_field_sclap_attr(&field) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            assert!(err.to_string().contains("requires `#[sclap(...)]"));
        }

        #[test]
        fn errors_when_multiple_sclap_attrs_exist() {
            let field = parse_named_field(quote::quote!(
                #[sclap(validator = validators::a)]
                #[sclap(validator = validators::b)]
                name: String
            ));

            let err = match parse_field_sclap_attr(&field) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            assert!(
                err.to_string()
                    .contains("multiple `#[sclap(...)]` attributes are not allowed")
            );
        }
    }

    // parse_struct_fields: whole-struct aggregation behavior.
    mod parse_struct_fields_tests {
        use super::*;

        #[test]
        fn parses_explicit_key_and_validator() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(key = "user.name", validator = validators::name)]
                    name: String,
                }
            };
            let fields = fields_of(input);
            let cfg = parse_struct_fields(&fields).expect("should parse");

            assert_eq!(cfg.keys, vec!["user.name".to_string()]);
            let (ident, path) = cfg
                .validator_map
                .get("user.name")
                .expect("validator should exist");
            assert_eq!(ident.to_string(), "name");
            let expected_path: Path = parse_quote!(validators::name);
            assert_eq!(path, &expected_path);
        }

        #[test]
        fn uses_field_name_as_default_key() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(validator = validators::mode)]
                    mode: String,
                }
            };
            let fields = fields_of(input);
            let cfg = parse_struct_fields(&fields).expect("should parse");

            assert_eq!(cfg.keys, vec!["mode".to_string()]);
            assert!(cfg.validator_map.contains_key("mode"));
            let (ident, path) = cfg.validator_map.get("mode").unwrap();
            assert_eq!(ident.to_string(), "mode");
            let expected_path: Path = parse_quote!(validators::mode);
            assert_eq!(path, &expected_path);
        }

        #[test]
        fn errors_when_sclap_attribute_is_missing() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    name: String,
                }
            };
            let fields = fields_of(input);
            let err = match parse_struct_fields(&fields) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            let msg = err.to_string();

            assert!(msg.contains("requires `#[sclap(...)]"));
        }

        #[test]
        fn errors_when_validator_is_missing() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(key = "name")]
                    name: String,
                }
            };
            let fields = fields_of(input);
            let err = match parse_struct_fields(&fields) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            let msg = err.to_string();

            assert!(msg.contains("Missing `validator` attribute"));
        }

        #[test]
        fn errors_when_multiple_sclap_attributes_exist_on_field() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(validator = validators::a)]
                    #[sclap(validator = validators::b)]
                    name: String,
                }
            };
            let fields = fields_of(input);
            let err = match parse_struct_fields(&fields) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            let msg = err.to_string();

            assert!(msg.contains("multiple `#[sclap(...)]` attributes are not allowed"));
        }

        #[test]
        fn errors_when_unknown_sclap_property_is_used() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(foo = "x", validator = validators::a)]
                    name: String,
                }
            };
            let fields = fields_of(input);
            let err = match parse_struct_fields(&fields) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            let msg = err.to_string();

            assert!(msg.contains("unsupported sclap attribute property"));
        }

        #[test]
        fn errors_when_keys_are_duplicated() {
            let input: DeriveInput = parse_quote! {
                struct Demo {
                    #[sclap(key = "same", validator = validators::a)]
                    first: String,
                    #[sclap(key = "same", validator = validators::b)]
                    second: String,
                }
            };
            let fields = fields_of(input);
            let err = match parse_struct_fields(&fields) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            let msg = err.to_string();

            assert!(msg.contains("Duplicate key `same` found"));
        }

        #[test]
        fn errors_for_tuple_structs() {
            let input: DeriveInput = parse_quote! {
                struct Demo(
                    #[sclap(validator = validators::a)] String
                );
            };
            let fields = fields_of(input);
            let err = match parse_struct_fields(&fields) {
                Ok(_) => panic!("should fail"),
                Err(err) => err,
            };
            let msg = err.to_string();

            assert!(msg.contains("Tuple structs are not supported"));
        }
    }
}
