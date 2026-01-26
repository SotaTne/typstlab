//! システム全体で使用される定数定義

/// 検索に関連する制限設定
pub mod search {
    /// 1回の検索で返す最大マッチ数
    pub const MAX_MATCHES: usize = 50;

    /// 1ファイルあたりの最大マッチ数
    pub const MAX_MATCHES_PER_FILE: usize = 3;

    /// 検索対象とするファイルの最大サイズ（バイト）
    /// 1MB
    pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

    /// 検索時にスキャンする最大ファイル数
    pub const MAX_SCAN_FILES: usize = 1000;
}

/// 取得に関連する制限設定
pub mod get {
    /// 1ページあたりの最大行数
    pub const MAX_GET_LINES: usize = 100;
}
