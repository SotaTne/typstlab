use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

const CORE_PACKAGE_NAME: &str = "sclap-core";
const CORE_SNAKE_PACKAGE_NAME: &str = "sclap_core";

pub(crate) fn core_crate_path() -> TokenStream2 {
    use proc_macro_crate::{FoundCrate, crate_name};

    let resolved_core_crate_name = match crate_name(CORE_PACKAGE_NAME) {
        Ok(FoundCrate::Name(name)) => Some(name),
        _ => None,
    };
    let current_expanding_package_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
    core_crate_path_from(
        resolved_core_crate_name.as_deref(),
        &current_expanding_package_name,
    )
}

fn core_crate_path_from(
    resolved_core_crate_name: Option<&str>,
    current_expanding_package_name: &str,
) -> TokenStream2 {
    if let Some(name) = resolved_core_crate_name {
        let ident = format_ident!("{}", name);
        return quote!(::#ident);
    }

    if current_expanding_package_name == CORE_PACKAGE_NAME {
        return quote!(crate);
    }

    let snake_ident = format_ident!("{}", CORE_SNAKE_PACKAGE_NAME);
    quote!(::#snake_ident)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn returns_found_crate_name_when_detected() {
        let path = core_crate_path_from(Some("renamed_sclap_core"), "any-package");
        assert_eq!(path.to_string(), quote!(::renamed_sclap_core).to_string());
    }

    #[test]
    fn returns_crate_when_current_package_is_sclap_core() {
        let path = core_crate_path_from(None, "sclap-core");
        assert_eq!(path.to_string(), quote!(crate).to_string());
    }

    #[test]
    fn falls_back_to_snake_case_package_name() {
        let path = core_crate_path_from(None, "another-package");
        assert_eq!(path.to_string(), quote!(::sclap_core).to_string());
    }
}
