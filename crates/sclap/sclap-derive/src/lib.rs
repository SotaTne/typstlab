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

mod sclap_context;
mod sclap_context_cfg;
mod utils;
