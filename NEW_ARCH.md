# Typstlab 新アーキテクチャ設計書 (NEW_ARCH.md)

このドキュメントは、Typstlab を「矛盾（事故）が構造的に発生しない、堅牢な OSS」として再定義するための設計指針である。Rails、Spring Boot、Terraform の設計思想を、Rust の型安全性とファイルシステム操作というドメインに最適化して融合させている。

---

## 1. コア・ストーリー： 「実体（Reality）」と「語り（Speaker）」の統合

### 課題： 「看板と中身の矛盾」
従来のアーキテクチャでは、操作ロジック（関数）と表示ロジック（ログ）が分離されていた。これにより、「ログでは PDF を生成したと表示されているが、実際には別のパスに出力されている」といった**計算上のズレ（矛盾）**が発生するリスクがあった。

### 解決策： すべてを実体（Model）として扱う
この新アーキテクチャでは、ファイル、ディレクトリ、さらには「実行結果」すらも**実体（Model）**として定義する。
1.  **実体（Model）**は、自分の居場所（Path）と正しさ（Validation）を知っている。
2.  **動き（Action）**は、実体と実体を掛け合わせ、新しい実体（結果）を生む。
3.  **表現（Speaker）**は、モデル自身が実装するプロトコルであり、自分が持つ実体のパスを直接参照して語る。

これにより、情報の発生源が常に一つのオブジェクトに集約され、矛盾の発生を物理的に不可能にする。

---

## 2. フォルダ構造と責務

```text
crates/
├── typstlab-proto/          # 【法 / Protocols】
│   └── src/                 # 依存を持たない、純粋なインターフェース（Trait）定義
│       ├── entity.rs        # trait Entity { path(), exists() }
│       ├── speaker.rs       # trait CliSpeaker, trait McpSpeaker
│       └── validator.rs     # trait Validatable { validate() }
│
├── typstlab-base/           # 【地盤 / Foundation】
│   └── src/                 # プロトコルを支える抽象的な実装基盤
│       ├── persistence/     # ファイルI/O、アトミック書き込み、ロック機構
│       ├── rendering/       # CLIの色テーマ、MCPのMarkdown構造定義
│       └── driver/          # Typstバイナリの純粋な実行ドライバ
│
├── typstlab-app/            # 【実体定義層 / Application】
│   └── src/
│       ├── models/          # プロトコルを具現化した実体そのもの
│       │   ├── project.rs   # Projectモデル (typstlab.toml)
│       │   ├── paper.rs     # Paperモデル (papers/ディレクトリ)
│       │   └── build_task.rs# BuildTaskモデル (実行結果の実体化)
│       └── actions/         # 実体を変容させる「動き」の定義
│           ├── build.rs     # BuildAction: Project+Paper -> BuildTask
│           └── scaffold.rs  # ScaffoldAction: 新しい実体を物理生成する
│
├── typstlab/                # 【CLIエントリポイント】薄い層
└── typstlab-mcp/            # 【MCPエントリポイント】薄い層
```

---

## 3. 各層の設計規約

### 3.1. typstlab-proto (法)
- **依存禁止:** 外部クレートや他レイヤーに依存してはならない。
- **純粋性:** 具象構造体（struct）は持たず、トレイト（trait）のみを定義する。
- **物理法則:** `Entity` トレイトは「実体があるなら必ずパスを持つ」というこの世界の物理法則を定義する。

### 3.2. typstlab-base (地盤)
- **汎用性:** `Paper` や `Project` といった具体的な名前を知ってはならない。
- **抽象化:** `Entity` トレイトを実装しているものなら、何であれロックしたり保存したりできる仕組みを提供する。

### 3.3. typstlab-app (実体)
- **単一の真実:** パス計算ロジックは必ず `Entity::path()` に実装し、他の場所でパスを組み立ててはならない。
- **Speaker の実装:** 
    - `BuildAction` は `CliSpeaker` と `McpSpeaker` を実装する。
    - 進行状況や結果を表示する際は、必ず自身のフィールドに持つ `Model`（Paper等）のメソッドから情報を取得する。
- **エラーの実体化:** 失敗すらも `BuildError` モデルとして実体化し、`Speaker` を通じて報告する。

### 3.4. typstlab / typstlab-mcp (入口)
- **薄さの徹底:** ロジックを一切持たず、ユーザー入力を `typstlab-app` の `Action` に変換し、`render_cli()` または `render_mcp()` を呼び出すだけに徹する。

---

## 4. 実装プロセス (BuildAction を例に)

1.  **初期化:** CLI が `BuildAction` を生成。この時、対象の `Paper` モデルを注入する。
2.  **検証:** `Action` が `validator.validate()` を実行。`Paper` の `main.typ` が存在するか確認。
3.  **語り(開始):** `action.render_progress()` を呼び出し、人間/AI に開始を告げる。
4.  **変容:** `Action` が `base::driver` を使い Typst を実行。
5.  **生成:** 実行結果から `BuildTask` モデル（成果物パス、成否、ログを持つ）を生成。
6.  **語り(結果):** `BuildTask` の情報を元に `action.render_result()` が最終報告を行う。

---

## 5. ラストチェック・マニフェスト

新しい機能を実装する際は、常に以下を自問自答すること：
- **「それ」は実体か？** (ファイル、フォルダ、または実行結果という事実か)
- **「それ」はプロトコルに従っているか？** (Entity, Speaker, Validator)
- **表示と実態がズレる余地はないか？** (パスの発生源は一つか)

このアーキテクチャを遵守することで、Typstlab は AI と人間が等しく信頼できる、最高品質のドキュメント制作プラットフォームとなる。
