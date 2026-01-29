use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

/// Resolves the path to the `typstlab-lsp-core` crate.
///
/// This function handles three scenarios:
/// 1. The crate is being used as a dependency (handles renaming via `proc-macro-crate`).
/// 2. We are inside the `typstlab-lsp-core` crate itself (returns `crate`).
/// 3. Fallback for other cases (returns `::typstlab_lsp_core`).
pub fn core_crate_path() -> TokenStream2 {
    use proc_macro_crate::{FoundCrate, crate_name};

    // 1. proc-macro-crate で検索
    if let Ok(FoundCrate::Name(name)) = crate_name("typstlab-lsp-core") {
        let ident = format_ident!("{}", name);
        return quote!(::#ident);
    }

    // 2. 自身（typstlab-lsp-core）の中なら `crate`
    // Cargo がセットする環境変数で判定（テスト時などに有用）
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
    if pkg_name == "typstlab-lsp-core" {
        return quote!(crate);
    }

    // 3. フォールバック（通常はこれで動くはず）
    quote!(::typstlab_lsp_core)
}
