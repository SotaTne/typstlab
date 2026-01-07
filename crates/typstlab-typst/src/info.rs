use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum TypstSource {
    Managed,
    System,
    InstalledRelease,
    InstalledCargo,
}

impl fmt::Display for TypstSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypstSource::Managed => write!(f, "managed cache"),
            TypstSource::System => write!(f, "system PATH"),
            TypstSource::InstalledRelease => write!(f, "installed release"),
            TypstSource::InstalledCargo => write!(f, "installed cargo"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypstInfo {
    pub version: String,
    pub source: TypstSource,
    pub path: PathBuf,
}
