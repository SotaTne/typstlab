# typstlab-typst

typstlabのためのTypstバイナリ解決とコマンド実行機能。

## 概要

このクレートは以下の機能を提供します：

- 複数のソース（管理キャッシュ、システムPATH）からTypstバイナリを解決
- 解決されたバイナリでTypstコマンドを実行
- バイナリのバージョン検証

## アーキテクチャ

クレートは3つの主要なモジュールで構成されています：

- **`info`**: バイナリのメタデータを表すコア型（`TypstInfo`、`TypstSource`）
- **`resolve`**: 多段階検索戦略によるバイナリ解決ロジック
- **`exec`**: 出力キャプチャとタイミング測定を伴うコマンド実行

## バイナリ解決フロー

```text
resolve_typst()
    ↓
1. キャッシュをチェック（force_refreshが無効の場合）
    ↓ (キャッシュミス)
2. 管理キャッシュを試行
    → 検索: {cache_dir}/{version}/typst
    → バージョン検証
    ↓ (見つからない)
3. システムPATHを試行
    → which::which("typst")を使用
    → バージョン検証
    ↓ (見つからない)
4. 検索した場所と共にNotFoundを返す
```

## 使用例

### 基本的な使い方

```rust
use typstlab_typst::{resolve_typst, exec_typst, ResolveOptions, ExecOptions};
use std::path::PathBuf;

// Typstバイナリを解決
let resolve_opts = ResolveOptions {
    required_version: "0.17.0".to_string(),
    project_root: PathBuf::from("."),
    force_refresh: false,
};

let result = resolve_typst(resolve_opts)?;

// Typstコマンドを実行
let exec_opts = ExecOptions {
    project_root: PathBuf::from("."),
    args: vec!["compile".to_string(), "document.typ".to_string()],
    required_version: "0.17.0".to_string(),
};

let exec_result = exec_typst(exec_opts)?;
println!("終了コード: {}", exec_result.exit_code);
println!("出力: {}", exec_result.stdout);
```

### 解決結果の処理

```rust
use typstlab_typst::{resolve_typst, ResolveOptions, ResolveResult};
use std::path::PathBuf;

let opts = ResolveOptions {
    required_version: "0.17.0".to_string(),
    project_root: PathBuf::from("."),
    force_refresh: false,
};

match resolve_typst(opts)? {
    ResolveResult::Cached(info) => {
        println!("キャッシュで見つかりました: {:?}", info.path);
    }
    ResolveResult::Resolved(info) => {
        println!("{}から解決されました: {:?}", info.source, info.path);
    }
    ResolveResult::NotFound { required_version, searched_locations } => {
        println!("バージョン{}が見つかりませんでした", required_version);
        println!("検索場所: {:?}", searched_locations);
    }
}
```

### 強制リフレッシュ

```rust
let opts = ResolveOptions {
    required_version: "0.17.0".to_string(),
    project_root: PathBuf::from("."),
    force_refresh: true,  // キャッシュをバイパス
};

let result = resolve_typst(opts)?;
```

### カスタム引数での実行

```rust
let exec_opts = ExecOptions {
    project_root: PathBuf::from("."),
    args: vec![
        "compile".to_string(),
        "--format".to_string(),
        "pdf".to_string(),
        "document.typ".to_string(),
    ],
    required_version: "0.17.0".to_string(),
};

let result = exec_typst(exec_opts)?;

if result.exit_code == 0 {
    println!("成功！ {}ms かかりました", result.duration_ms);
} else {
    eprintln!("エラー: {}", result.stderr);
}
```

## 管理キャッシュの構造

Typstバイナリは以下のOS固有の場所にキャッシュされます：

- **macOS**: `~/Library/Caches/typstlab/typst/{version}/typst`
- **Linux**: `~/.cache/typstlab/typst/{version}/typst`
- **Windows**: `%LOCALAPPDATA%\typstlab\typst\{version}\typst.exe`

## エラーハンドリング

全ての関数は`typstlab-core`から`Result<T, TypstlabError>`を返します：

```rust
use typstlab_core::{Result, TypstlabError};

fn example() -> Result<()> {
    let opts = ExecOptions { /* ... */ };

    match exec_typst(opts) {
        Ok(result) => {
            println!("成功: {}", result.stdout);
        }
        Err(TypstlabError::TypstNotResolved { required_version }) => {
            eprintln!("バージョン{}のバイナリが見つかりませんでした", required_version);
        }
        Err(e) => {
            eprintln!("エラー: {:?}", e);
        }
    }

    Ok(())
}
```

## テスト

クレートには包括的なテストが含まれています：

- **ユニットテスト**: 全ての関数とエッジケースをカバーする31テスト
- **統合テスト**: 完全なワークフローを検証する6つのE2Eテスト

テストの実行方法：

```bash
# 全てのテスト
cargo test -p typstlab-typst -- --test-threads=1

# ユニットテストのみ
cargo test -p typstlab-typst --lib -- --test-threads=1

# 統合テストのみ
cargo test -p typstlab-typst --test integration_test -- --test-threads=1
```

**注意**: テストは管理キャッシュディレクトリを共有するため、順次実行する必要があります（`--test-threads=1`）。

## 機能

- ✅ 多段階バイナリ解決（キャッシュ → 管理 → システム）
- ✅ 厳密なバージョン検証
- ✅ クロスプラットフォーム対応（macOS、Linux、Windows）
- ✅ stdout/stderrのキャプチャ
- ✅ 終了コードの保持
- ✅ 実行時間の測定
- ✅ 包括的なエラーハンドリング
- ✅ 100%テストカバレッジ

## 依存関係

- `which` - システムPATH内のバイナリを検索
- `dirs` - OS固有のキャッシュディレクトリ解決
- `serde` - シリアライゼーションサポート
- `chrono` - タイムスタンプ処理
- `thiserror` - エラー型定義
- `typstlab-core` - 共有コア型とエラー

## ライセンス

ライセンス情報についてはワークスペースのルートを参照してください。
