use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;
use tempfile::{Builder, TempDir};

/// 物理的な永続化をアトミックに（失敗時に中途半端なゴミを残さず）行うためのユーティリティ
pub struct Persistence;

impl Persistence {
    /// ファイルをアトミックに書き込む。
    pub fn write_file<P: AsRef<Path>>(path: P, content: &[u8]) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut tmp = Builder::new().prefix(".typstlab-tmp-").tempfile_in(
            path.parent()
                .ok_or_else(|| anyhow!("No parent directory"))?,
        )?;

        use std::io::Write;
        tmp.write_all(content)?;
        tmp.persist(path)?;
        Ok(())
    }

    /// ディレクトリをアトミックに配置（確定）する。
    pub fn commit_directory<P: AsRef<Path>>(staging_path: P, dest_path: P) -> Result<()> {
        let staging = staging_path.as_ref();
        let dest = dest_path.as_ref();

        if dest.exists() {
            return Err(anyhow!(
                "Destination directory already exists: {}",
                dest.display()
            ));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(staging, dest).map_err(|e| {
            anyhow!(
                "Failed to commit directory from {} to {}: {}",
                staging.display(),
                dest.display(),
                e
            )
        })?;

        Ok(())
    }

    /// 一時的な作業用ディレクトリ（RAII）を作成する。
    pub fn create_temp_dir<P: AsRef<Path>>(base_dir: P, prefix: &str) -> Result<TempDir> {
        let base = base_dir.as_ref();
        fs::create_dir_all(base)?;

        let temp_dir = Builder::new().prefix(prefix).tempdir_in(base)?;

        Ok(temp_dir)
    }
}
