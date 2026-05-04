# typstlab

[English](README.md) | 日本語

`typstlab` は、Typst プロジェクトのためのツールチェーン管理ツールです。

## なぜ作ったか

Typst には素晴らしい本体と周辺ツールがあります。
しかし、Typst のバージョン互換を保ったまま、docs、build、MCP まで含めたツールチェーンとして扱えるものは多くありませんでした。

skills や Makefile を書いて対応することはできます。
ただ、それでも実行が遅かったり、git で管理しにくかったり、同じ環境を再現できるとは言い切れない問題が残りました。

現代のプログラミング言語では、formatter、LSP、test、build、docs が同じプロジェクトの中で自然に繋がっていることが重要です。
一方で Typst や周辺ツールでは、それぞれのツールがどの Typst バージョンと互換性を持つのかを確認し続けるのが難しく、その良さを十分に活かしきれない場面がありました。

`typstlab` は、その問題を解決するために作っています。
一番直接的な理由は、私自身が学校のレポートを書く上で、Typst を安定したツールチェーンとして使いたかったからです。

Typst 本体のバージョン、Typst のドキュメント、プロジェクト内の paper/template/dist の構造をまとめて扱い、同じ操作を CLI と MCP から使えるようにすることを目指しています。

まだ開発中ですが、現在は Typst の実行、ドキュメントの取得と変換、MCP 経由の操作が使えます。

## 外部プロジェクトとの関係

`typstlab` は、Typst 本体や Typst docs の成果物を利用します。
これらのプロジェクトを再実装するものではありません。

現在、主に次の成果物に依存しています。

- Typst 本体: [`typst/typst`](https://github.com/typst/typst) の release binary
- Typst docs: [`typst-community/dev-builds`](https://github.com/typst-community/dev-builds) の docs release artifact

`typstlab` はそれらを直接改変せず、バージョン解決、ダウンロード、キャッシュ、プロジェクトへの同期、MCP からの利用を担当します。

## 現在の対応状況

- [x] Typst のバージョン解決
- [x] Typst バイナリのダウンロードとキャッシュ
- [x] Typst コマンド実行
- [x] PDF / PNG / SVG / HTML ビルド
- [x] Typst docs のダウンロード
- [x] Typst docs の Markdown 変換
- [x] MCP server
- [ ] fmt
- [ ] LSP
- [ ] test

## インストール

### 開発版

現時点では crates.io には公開していません。
crates.io 公開までは GitHub から clone して使ってください。

```bash
git clone https://github.com/SotaTne/typstlab.git
cd typstlab
cargo install --path crates/typstlab-cli
```

SSH を使う場合:

```bash
git clone git@github.com:SotaTne/typstlab.git
cd typstlab
cargo install --path crates/typstlab-cli
```

インストールすると `typstlab` コマンドが使えるようになります。

```bash
typstlab --help
```

インストールせずに試す場合は、リポジトリ内で `cargo run --` を使えます。

```bash
cargo run -- --help
```

## はじめての使い方

新しい Typst プロジェクトを作ります。

```bash
typstlab new my-project
cd my-project
```

プロジェクトの状態を確認します。

```bash
typstlab status
```

paper をビルドします。

```bash
typstlab build
```

出力形式を指定することもできます。

```bash
typstlab build --pdf
typstlab build --png
typstlab build --svg
typstlab build --html
```

何も指定しない場合は PDF を出力します。

## typstlab.toml

`typstlab` のプロジェクトには `typstlab.toml` があります。

```toml
[project]
name = "my-project"
init_date = "2026-05-04"

[toolchain]
typst = "0.14.2"
typst_docs = "auto"
typstyle = "none"

[structure]
papers_dir = "papers"
dist_dir = "dist"
templates_dir = "templates"
```

## toolchain の指定

`typst` は必ず明示的なバージョンを書きます。

```toml
[toolchain]
typst = "0.14.2"
```

`typst_docs` や将来追加される周辺ツールは、次の 3 種類で指定できます。

```toml
typst_docs = "auto"
typst_docs = "none"
typst_docs = "0.14.2"
```

- `auto`: 互換性のある中で一番新しいバージョンを使う
- `none`: そのツールを使わない
- `"0.14.2"`: 指定したバージョンを使う

互換性のないバージョンを指定した場合はエラーになります。

## status

`status` は、今のプロジェクトがどの toolchain を使っているかを確認するコマンドです。

```bash
typstlab status
```

表示される主な情報:

- project name
- project root
- Typst version
- Typst binary path
- docs version
- docs path
- docs cache
- papers
- templates
- dist

例:

```text
typstlab status

Project
  name     my-project
  root     /path/to/my-project

Toolchain
  typst
    version  0.14.2
    binary   /path/to/cache/typst/0.14.2/typst
  docs
    version  0.14.2
    path     /path/to/my-project/.typstlab/typst_docs
    cache    /path/to/cache/docs
```

## build

`build` は `papers` ディレクトリ内の paper を Typst でビルドします。

```bash
typstlab build
```

特定の paper だけをビルドすることもできます。

```bash
typstlab build paper-id
```

複数指定もできます。

```bash
typstlab build paper-a paper-b
```

出力形式:

```bash
typstlab build --pdf
typstlab build --png
typstlab build --svg
typstlab build --html
```

## paper と template

paper を作成します。

```bash
typstlab gen paper intro
```

template を作成します。

```bash
typstlab gen template report
```

template を指定して paper を作成できます。

```bash
typstlab gen paper intro --template report
```

ローカル template が見つからない場合は、Typst の template 初期化にフォールバックします。

## Typst docs

`typst_docs = "auto"` にしている場合、`typstlab` は Typst のバージョンに対応する docs を解決します。

docs は一度 cache に保存され、プロジェクト側には `.typstlab/typst_docs` として同期されます。

```text
Global cache
└── typstlab/
    └── docs/
        └── 0.14.2/
            └── ...
                 ↓ sync
Project
└── .typstlab/
    └── typst_docs/
        └── ...
```

- 初回: docs をダウンロードして cache に保存
- 2 回目以降: cache からプロジェクトへ同期
- 別プロジェクト: 同じ docs cache を再利用

これにより、プロジェクトの中では解決済みの docs を参照でき、別プロジェクトでは同じ cache を再利用できます。

## MCP

MCP server は stdio で起動できます。

```bash
typstlab mcp stdio .
```

インストールせずに起動する場合:

```bash
cargo run -- mcp stdio .
```

現在 MCP から使える主な機能:

- project status の取得
- paper のビルドと PNG 画像の返却

これにより、エディタや agent から `typstlab` の状態確認やビルド実行を行えるようになります。

MCP クライアントには、プロジェクトルートを引数として渡します。

```json
{
  "mcpServers": {
    "typstlab": {
      "command": "typstlab",
      "args": ["mcp", "stdio", "/path/to/project"]
    }
  }
}
```

## トラブルシューティング

### どの cache を使っているか確認したい

`status` で Typst binary と docs cache の場所を確認できます。

```bash
typstlab status
```

docs の内容が古い、または壊れているように見える場合は、まず `status` で表示される `cache` の場所を確認してください。

### docs の同期をやり直したい

docs は cache からプロジェクト内の `.typstlab/typst_docs` に同期されます。
同期先だけを作り直したい場合は、プロジェクト内の `.typstlab/typst_docs` を削除してから `status` を実行してください。

```bash
typstlab status
```

## このツールが目指しているもの

Typst は単体でも強力ですが、実際のプロジェクトでは次のような問題が出ます。

- どの Typst バージョンでビルドするか
- docs のバージョンをどう揃えるか
- paper や template をどう管理するか
- agent や editor からどう安全にビルドするか
- 将来的に fmt / LSP / test をどう統合するか

`typstlab` は、これらを 1 つの project toolchain として扱うためのレイヤーです。

現時点ではまだ完成品ではありません。
まずは Typst コマンド実行、docs、MCP を安定させ、その上に fmt、LSP、test を追加していく予定です。
