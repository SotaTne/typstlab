use std::path::{Component, Path, PathBuf};

/// パスが領土内（相対パス）かつ安全であることをクロスプラットフォームで検証する。
/// 以下のいずれかに該当する場合は `false` を返す。
/// 1. パスの開始が Unix 形式または Windows 形式のルート参照 (`/`, `\`, etc.) である。
/// 2. パスの開始が Windows のドライブプレフィックス (`C:`, etc.) である。
/// 3. 親ディレクトリへの移動 (`..`) コンポーネントを含む。
pub fn is_path_safe(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // 補助的な文字列スキャン (物理的な Component 分割の前にルート/プレフィックスを遮断)
    if path_str.starts_with('/')
        || path_str.starts_with('\\')
        || has_windows_drive_prefix(&path_str)
    {
        return false;
    }

    // コンポーネントレベルでの精密検査
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return false,
            _ => {}
        }
    }

    true
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// パスの先頭から指定された数のコンポーネントを剥ぎ取る。
pub fn strip_path(path: &Path, count: usize) -> Option<PathBuf> {
    let mut components = path.components();
    for _ in 0..count {
        components.next()?;
    }
    let stripped = components.as_path();
    if stripped.as_os_str().is_empty() {
        None
    } else {
        Some(stripped.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_safe_alignment() {
        // 正常系: 合法なファイル名
        assert!(is_path_safe(Path::new("file.txt")));
        assert!(is_path_safe(Path::new("version..1.typ"))); // 合法
        assert!(is_path_safe(Path::new("dir/subdir/file.bin")));
        // 中間の ':' は Unix では合法
        assert!(is_path_safe(Path::new("foo:bar.txt")));

        // 異常系: 絶対パス
        assert!(!is_path_safe(Path::new("/etc/passwd")));

        // 異常系: トラバーサル (..)
        assert!(!is_path_safe(Path::new("../secret.txt")));
        assert!(!is_path_safe(Path::new("a/../../b")));

        // 異常系: Windows 形式 (Rooted/Prefix)
        assert!(!is_path_safe(Path::new("\\temp\\evil")));
        assert!(!is_path_safe(Path::new("C:\\Windows")));
        assert!(!is_path_safe(Path::new("D:/data")));
    }

    #[test]
    fn test_strip_path() {
        let p = Path::new("a/b/c.txt");
        assert_eq!(strip_path(p, 1).unwrap(), Path::new("b/c.txt"));
        assert!(strip_path(p, 3).is_none());
    }
}
