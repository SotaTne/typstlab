use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum TypstSource {
    Managed,
    System,
    InstalledRelease,
    InstalledCargo,
}

#[derive(Debug, Clone)]
pub struct TypstInfo {
    pub version: String,
    pub source: TypstSource,
    pub path: PathBuf,
}
