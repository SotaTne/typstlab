mod error;
pub mod html;
pub mod md;
mod render;
mod route;
mod schema;
mod sink;

pub use crate::docs::RenderedDocs;
pub use error::DocsRenderError;
pub use render::render_docs_from_reader;
pub use schema::DocsEntry;
