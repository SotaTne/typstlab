use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// state.json schema - 破棄可能な実行状態キャッシュ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub schema_version: String,
    pub machine: MachineInfo,
    #[serde(default)]
    pub typst: Option<TypstState>,
    #[serde(default)]
    pub docs: Option<DocsState>,
    #[serde(default)]
    pub uv: Option<UvState>,
    #[serde(default)]
    pub build: Option<BuildState>,
    #[serde(default)]
    pub sync: Option<SyncState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineInfo {
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypstState {
    pub resolved_path: PathBuf,
    pub resolved_version: String,
    pub resolved_source: ResolvedSource,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResolvedSource {
    Managed,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsState {
    pub typst: Option<TypstDocsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypstDocsInfo {
    pub present: bool,
    pub version: String,
    pub synced_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UvState {
    pub resolved_path: PathBuf,
    pub resolved_version: String,
    pub resolved_source: ResolvedSource,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildState {
    pub last: Option<LastBuild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastBuild {
    pub paper: String,
    pub success: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub output: PathBuf,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub last_sync: Option<DateTime<Utc>>,
}

impl State {
    /// 空の State を作成
    pub fn empty() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            machine: MachineInfo::detect(),
            typst: None,
            docs: None,
            uv: None,
            build: None,
            sync: None,
        }
    }

    /// state.json を読み込む
    pub fn load(path: impl AsRef<Path>) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            crate::error::TypstlabError::StateReadError(format!("Failed to read: {}", e))
        })?;

        let state: Self = serde_json::from_str(&content).map_err(|e| {
            crate::error::TypstlabError::StateReadError(format!("Failed to parse: {}", e))
        })?;

        // Schema evolution
        match state.schema_version.as_str() {
            "1.0" => Ok(state),
            version => Err(crate::error::TypstlabError::StateInvalidSchema(
                version.to_string(),
            )),
        }
    }

    /// state.json に書き込む（原子的更新）
    ///
    /// This method uses atomic file updates to prevent corruption:
    /// 1. Acquire exclusive lock on .typstlab/.lock
    /// 2. Write to temporary file (using tempfile crate)
    /// 3. Fsync temporary file and parent directory
    /// 4. Atomic persist (cross-platform, Windows compatible)
    /// 5. Release lock (automatic via RAII)
    pub fn save(&self, path: impl AsRef<Path>) -> crate::error::Result<()> {
        let path = path.as_ref();
        let parent = ensure_parent_dir(path)?;
        let _lock = acquire_state_lock(parent)?;
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::TypstlabError::StateWriteError(format!("Failed to serialize: {}", e))
        })?;
        atomic_write_json(&content, path, parent)?;
        Ok(())
    }

    /// state.json が存在すれば読み込み、なければ空の State を返す
    pub fn load_or_empty(path: impl AsRef<Path>) -> Self {
        Self::load(&path).unwrap_or_else(|_| Self::empty())
    }
}

/// Ensure parent directory exists and return it
fn ensure_parent_dir(path: &Path) -> crate::error::Result<&Path> {
    let parent = path.parent().ok_or_else(|| {
        crate::error::TypstlabError::StateWriteError(
            "State path has no parent directory".to_string(),
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        crate::error::TypstlabError::StateWriteError(format!("Failed to create parent dir: {}", e))
    })?;
    Ok(parent)
}

/// Acquire exclusive lock on .typstlab/.lock
fn acquire_state_lock(parent: &Path) -> crate::error::Result<crate::lock::LockGuard> {
    let lock_path = parent.join(".lock");
    crate::lock::acquire_lock(
        &lock_path,
        std::time::Duration::from_secs(30),
        "state update",
    )
    .map_err(|e| {
        crate::error::TypstlabError::StateWriteError(format!("Failed to acquire lock: {}", e))
    })
}

/// Write JSON atomically using NamedTempFile + persist (Windows compatible)
fn atomic_write_json(content: &str, path: &Path, parent: &Path) -> crate::error::Result<()> {
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new_in(parent).map_err(|e| {
        crate::error::TypstlabError::StateWriteError(format!("Failed to create temp file: {}", e))
    })?;

    temp_file.write_all(content.as_bytes()).map_err(|e| {
        crate::error::TypstlabError::StateWriteError(format!("Failed to write temp file: {}", e))
    })?;

    temp_file.as_file().sync_all().map_err(|e| {
        crate::error::TypstlabError::StateWriteError(format!("Failed to sync temp file: {}", e))
    })?;

    temp_file.persist(path).map_err(|e| {
        crate::error::TypstlabError::StateWriteError(format!("Failed to persist temp file: {}", e))
    })?;

    // Fsync parent directory for durability (Unix only)
    #[cfg(unix)]
    {
        let parent_file = File::open(parent).map_err(|e| {
            crate::error::TypstlabError::StateWriteError(format!(
                "Failed to open parent dir: {}",
                e
            ))
        })?;
        parent_file.sync_all().map_err(|e| {
            crate::error::TypstlabError::StateWriteError(format!(
                "Failed to sync parent dir: {}",
                e
            ))
        })?;
    }

    Ok(())
}

impl MachineInfo {
    /// 現在のマシン情報を検出
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state() {
        let state = State::empty();
        assert_eq!(state.schema_version, "1.0");
        assert!(state.typst.is_none());
        assert!(state.uv.is_none());
        assert!(state.sync.is_none());
    }

    #[test]
    fn test_state_serialization() {
        let state = State::empty();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: State = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.schema_version, "1.0");
    }

    #[test]
    fn test_machine_detection() {
        let machine = MachineInfo::detect();
        assert!(!machine.os.is_empty());
        assert!(!machine.arch.is_empty());
    }
}
