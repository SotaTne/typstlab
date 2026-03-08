//! #[Derive(Sclap)]
//! struct CustomSclap {
//!    typst: SclapTypst,
//! }
//!
//! #[Derive(Sclap)]
//! struct SclapTypst {
//!    compile: SclapTypstCompile,
//!    run: SclapTypstRun,
//! }
//!
//! #[Derive(Sclap)]
//! struct SclapTypstCompile {
//!    #[guard(ここで、バージョンのチェックを行う)]
//!    mode:
//! }
//!
//!
//!
//!
//! let sclap:Sclap<SclapConfig> = CustomSclap::new({
//!     version: "0.1.0".to_string(),
//!     output: "json".to_string(),
//! });

mod sclap_validator;
mod sclap_validator_cfg;
mod utils;

use proc_macro::TokenStream;

#[proc_macro_derive(SclapValidator, attributes(sclap))]
pub fn derive_feature(input: TokenStream) -> TokenStream {
    sclap_validator::derive_sclap_validator(input)
}
