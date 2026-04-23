use std::fs;
use std::path::Path;
use anyhow::Result;

pub struct AtomicFile;

impl AtomicFile {
    pub fn write<P: AsRef<Path>>(path: P, content: &[u8]) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // シンプルな原子書き込みのプロトタイプ
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(temp_path, path)?;
        Ok(())
    }
}
