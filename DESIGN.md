# typstlab Design Document

**Version**: 0.1.0
**Last Updated**: 2026-01-05

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Directory Structure](#2-directory-structure)
3. [Schema Definitions](#3-schema-definitions)
4. [Layouts System](#4-layouts-system)
5. [Command Specifications](#5-command-specifications)
6. [System Design](#6-system-design)
7. [Implementation Guide](#7-implementation-guide)

---

## 1. Project Overview

### 1.1 Purpose

typstlab は「Typst を使って論文・レポートを書く行為」を、

- **再現可能**
- **機械可読**
- **エージェント実行可能**

な **フレームワーク化されたビルド対象** として扱うための、薄いカーネル集合型CLI である。

これは単なる Typst の便利ツールではない。LaTeX 環境や ad-hoc な Typst プロジェクトが抱える

- 環境が再現できない
- 図表・データ生成が壊れる
- 引用の出典が曖昧
- AI エージェントが安全に操作できない

という構造的問題を **規約・状態・契約** によって解決することを目的とする。

### 1.2 Design Principles (Non-negotiable)

#### 1.2.1 プロジェクトはフレームワークである

- ディレクトリ構造は「慣習」ではなく **仕様**
- 正（source of truth）は常に1箇所
- 自由度よりも **機械可読性・安全性** を優先

#### 1.2.2 AI エージェント前提

typstlab は人間だけでなく **コードエージェントが操作する前提** で設計される。そのため以下を必須とする：

- 状態を JSON で取得できる（`status --json`）
- 副作用（network / writes / reads）を明示
- 危険な操作は typstlab 経由でしか実行できない

#### 1.2.3 管理しないが観測する

- Typst のパッケージ管理は Typst 本体に任せる
- ただし「何が使われたか」は **観測・記録** する

#### 1.2.4 rules/ は AI agent の「知識」

- `rules/` には Markdown 形式で自由に文書を配置できる
- AI agent は MCP tools 経由で rules を参照し、プロジェクトの文脈を理解する
- SOT ではないが、再現性・説明可能性のために git commit を推奨

### 1.3 Scope (v0.1)

**含むもの**：

- PDF ビルド（`build` コマンド）
- プロジェクト / paper / status の骨格
- Typst バージョン固定（プロジェクト単位）
- Typst 管理（install, docs, sync）
- Layout 生成システム（`gen` コマンド、`_generated/`）
- プロジェクト管理（`new`, `init`, `setup` コマンド）
- 診断機能（`doctor`, `status` コマンド）
- MCP サーバ / LSP (Stub)

**v0.2以降に延期**：

- watch 最適化（`watch` コマンド）
- uv 統合（exec）
- refs 管理（fetch/check/touch）と sources.lock
- `gen lib`, `gen layout`

**含まないもの**：

- docs 検索
- HTML 出力
- パッケージ情報収集
- GUI

### 1.4 Non-Goals

- 初心者向けツールにはしない
- 魔法の自動論文生成はしない
- 規約を緩めない

### 1.5 One-line Definition

> **typstlab is a reproducible, agent-ready paper framework for Typst.**

---

## 2. Directory Structure

### 2.1 Standard Project Structure

```
project/
  typstlab.toml          # プロジェクト全体の規約（正）
  pyproject.toml         # Python環境宣言（正）（v0.2以降）
  uv.lock                # Python依存ロック（正）（v0.2以降）

  layouts/               # プロジェクトレベルのレイアウト定義
    default/
      meta.tmp.typ       # テンプレートファイル（{{ }} プレースホルダー）
      header.typ         # 静的ファイル（プレースホルダーなし）
      refs.tmp.typ       # テンプレートファイル
    ieee/
      meta.tmp.typ
      header.typ
      refs.tmp.typ

  refs/                  # （v0.2以降で実装予定）
    sets/
      <set-id>/
        library.bib
        sources.lock     # 取得日ベース（+可能ならhash）

  scripts/               # 図表・表の生成スクリプト置き場（実行はv0.2以降）
  data/                  # 入力データ（原則immutable）
  figures/               # 生成物（図）

  rules/                 # AI agent向け参考情報（Markdown）
    paper/               # 論文執筆の規約・ガイド
    scripts/             # スクリプト実装の補足
    data/                # データ形式の説明
    misc/                # その他
    README.md            # このディレクトリの説明

  dist/                  # 出力集約先（規約上必須、内容は派生物）
    <paper-id>/
      <output_name>.pdf

  papers/
    <paper-id>/
      paper.toml         # paperメタ情報（正）
      main.typ           # このpaperのエントリポイント（正）
      _generated/        # 自動生成（編集禁止）
        meta.typ
        header.typ
        refs.typ
      sections/          # splitモード時の本文
      assets/            # paper固有のアセット
      rules/             # paper固有の参考情報
        README.md

  bin/                   # shim（bin/typst, bin/uv 等）
    typst
    uv

  .typstlab/
    kb/
      typst/
        docs/            # Typst docs (MD) の実体
    logs/
    state.json           # 実行状態キャッシュ（破棄可能）
```

### 2.2 Source of Truth（正の定義）

| 項目 | 分類 | ファイル | 備考 |
|------|------|----------|------|
| プロジェクト規約 | **正** | `typstlab.toml` | git commit |
| paper 規約 | **正** | `papers/<id>/paper.toml` | git commit |
| paper 本文 | **正** | `papers/<id>/main.typ` | git commit |
| Python 環境 | **正** | `pyproject.toml`, `uv.lock` | git commit |
| 参考文献（セット） | **正** | `refs/sets/<set-id>/library.bib` | git commit |
| 参考文献履歴（セット） | **正** | `refs/sets/<set-id>/sources.lock` | git commit |
| レイアウト | **正** | `layouts/**/*.typ` | git commit |
| rules/ | **参考情報** | `rules/**/*.md`, `papers/<id>/rules/**/*.md` | git commit推奨、AI agent向け |
| 出力物 | **派生物** | `dist/**/*.pdf` | gitignore、再生成可能 |
| 生成コード | **派生物** | `papers/*/_generated/*.typ` | gitignore、再生成可能 |
| bin shim | **派生物** | `bin/typst`, `bin/uv` | gitignore、再生成可能 |
| Typst docs | **観測物** | `.typstlab/kb/typst/docs/` | gitignore、再取得可能 |
| 実行状態 | **観測キャッシュ** | `.typstlab/state.json` | gitignore、破棄可能 |

**再現性の原則**：
再現性の正は **設定・規約・入力ファイルのみ** で構成される。
出力物（PDF、`_generated/` 等）は常に **再生成可能な派生物** である。
観測物・観測キャッシュは、環境の状態を記録するが、再現性を決定しない。

**writes_sot の定義**：
`writes_sot = true` とは、「SOT と定義されたファイル（git commit 対象）を変更しうる操作」を意味する。
`writes_sot = false` は、派生物（dist/, _generated/, bin/）や観測物（.typstlab/ 以下）、観測キャッシュ（state.json）だけを書き換える操作である。

---

## 3. Schema Definitions

### 3.1 typstlab.toml

プロジェクト全体の規約を定義する。これが「要求（requirements）」の正となる。

```toml
# プロジェクトメタ情報
[project]
name = "my-research"
init_date = "2026-01-05"  # プロジェクト作成日

# 新規 paper 作成時のデフォルト著者
[project.default_author]
name = "Alice"
email = "alice@example.com"
affiliation = "University"  # optional

# 新規 paper 作成時のデフォルトレイアウト
[project.default_layout]
name = "default"

# Typst ツールチェーン（必須）
[typst]
version = "0.13.1"  # 完全一致要求

# 外部ツール（v0.2以降で実装予定）
# [tools]
# uv = { required = false }  # v0.2で実装予定

# 注: v0.1では uv 統合を延期
# v0.2で以下の機能を実装予定：
# - uv link（Python環境の解決と診断）
# - uv exec（スクリプト実行）
# - pyproject.toml / uv.lock の管理

# ネットワークポリシー
[network]
policy = "auto"  # "auto" | "never"

# ビルドのデフォルト設定
[build]
parallel = true  # 複数 paper の並行ビルド（将来）

# watch のデフォルト設定（v0.2以降で実装予定）
# [watch]
# debounce_ms = 500
# ignore = ["*.tmp", ".DS_Store", "*.swp"]

# test 設定（v0.1 では compile のみ）
[test]
out = ".typstlab/test-out"  # optional

[[test.cases]]
name = "components"
type = "compile"
file-patterns = ["tests/components/*.typ"]
```

**重要な原則**：

- `typstlab.toml` は **要求（requirements）** のみを記述
- 解決結果（resolved）は `state.json` に保存
- git commit する

### 3.2 paper.toml

個別の paper のメタ情報と出力設定を定義する。

```toml
# Paper メタ情報
[paper]
id = "report"  # ディレクトリ名と必ず一致（papers/report/）
title = "My Research Report"
language = "en"  # "en" | "ja" | ...
date = "2026-01-05"  # 論文に載せる記述日

# 著者（複数可能）
[[paper.authors]]
name = "Alice"
email = "alice@example.com"
affiliation = "University"  # optional

[[paper.authors]]
name = "Bob"
email = "bob@example.com"
affiliation = "Company"

# レイアウト設定
[layout]
theme = "ieee"  # layouts/ieee/ を使う（省略時は "default"）

# 将来の拡張例（v0.2以降）:
# variant = "two-column"      # テーマのバリエーション
# colors = "dark"              # カラースキーム
# version = "2.0"              # レイアウトバージョン
# [layout.options]             # テーマ固有のオプション
# header_height = "2cm"
# line_spacing = 1.5

# 出力設定
[output]
name = "report"  # 拡張子なし → dist/report/report.pdf

# ビルド設定
[build]
targets = ["pdf"]  # v0.1 では pdf のみ

# 参考文献セット（optional）
[refs]
sets = ["core", "report-2026q1"]  # refs/sets/<set-id>/ を参照
# 空配列可能。空なら _generated/refs.typ は "No bibliography specified"

# test 設定（optional, v0.1 では compile のみ）
[test]
out = ".typstlab/test-out"  # optional

[[test.cases]]
name = "paper-template"
type = "compile"
file-patterns = ["paper/report.typ"]
```

**重要な原則**：

- `paper.id` とディレクトリ名は必ず一致
- 不一致の場合はエラー
- `[refs]` セクションは optional
- template 情報は含めない（Typst に任せる）

### 3.3 state.json

このマシンで typstlab を正しく・高速に動かすための、**破棄可能な実行状態キャッシュ**。

**重要な原則**：
state.json は **このマシンにおける実行状態の観測キャッシュ** であり、
**単体で再現性や正しさを保証するものではない**。
常に `typstlab.toml`（要求）と突き合わせて評価される。

**含めてよいもの**：

- 絶対パス（マシン固有）
- 最終確認時刻
- 成功/失敗の履歴

**含めてはいけないもの**：

- 再現性を決定する情報（バージョン要求等）
- プロジェクト規約（typstlab.toml に属する）

```json
{
  "schema_version": "1.0",

  "machine": {
    "os": "darwin",
    "arch": "aarch64"
  },

  "typst": {
    "resolved_path": "/Users/foo/Library/Caches/typstlab/typst/0.13.1/typst",
    "resolved_version": "0.13.1",
    "resolved_source": "managed",
    "checked_at": "2026-01-05T10:12:00Z"
  },

  "docs": {
    "typst": {
      "present": true,
      "version": "0.13.1",
      "synced_at": "2026-01-05T10:13:20Z",
      "source": "official"  // typst/typst リポジトリの docs（5.7.5 参照）
    }
  },

  "uv": {
    "resolved_path": "/opt/homebrew/bin/uv",
    "resolved_version": "0.5.1",
    "resolved_source": "system",
    "checked_at": "2026-01-05T10:12:05Z"
  },

  "build": {
    "last": {
      "paper": "paper1",
      "success": true,
      "started_at": "2026-01-05T10:19:58Z",
      "finished_at": "2026-01-05T10:20:01Z",
      "duration_ms": 3200,
      "output": "dist/paper1/report.pdf",
      "error": null
    }
  }
}
```

**重要な原則**：

- 破棄可能（削除しても再生成される）
- gitignore に含める
- マシン固有の情報を含む
- `checked_at` は最後に確認した時刻（基本的に信頼、再検証は明示的に）

### 3.4 status --json

プロジェクトまたは paper の現在の状態を機械可読形式で返す。エージェント操作の心臓部。

#### 3.4.1 paper 指定時

```json
{
  "schema_version": "1.0",

  "project": {
    "name": "my-research",
    "root": "/Users/alice/projects/my-research",
    "config": {
      "typst_version": "0.13.1",
      "network_policy": "auto"
    }
  },

  "paper": {
    "id": "report",
    "title": "My Research Report",
    "main_typ": "papers/report/main.typ",
    "output": "dist/report/report.pdf"
  },

  "timestamp": "2026-01-05T12:34:56Z",

  "checks": [
    {
      "id": "typst_resolved",
      "name": "Typst available",
      "status": "ok",
      "message": "Typst 0.13.1 resolved",
      "details": {
        "version": "0.13.1",
        "source": "managed",
        "path": "/Users/alice/Library/Caches/typstlab/typst/0.13.1/typst"
      }
    },
    {
      "id": "paper_main_exists",
      "name": "Main file exists",
      "status": "ok",
      "message": "papers/report/main.typ found"
    },
    {
      "id": "refs_issues",
      "name": "References",
      "status": "warning",
      "message": "2 undefined entries",
      "details": {
        "undefined": ["smith2020", "jones2021"]
      }
    }
  ],

  "actions": [
    {
      "id": "install_typst",
      "command": "typstlab typst install 0.13.1",
      "description": "Install required Typst version",
      "enabled": true,
      "safety": {
        "network": true,
        "writes": true,
        "writes_sot": false,
        "reads": true
      },
      "prerequisite": null
    },
    {
      "id": "build_paper",
      "command": "typstlab build --paper report",
      "description": "Build this paper to PDF",
      "enabled": false,
      "disabled_reason": "Typst 0.13.1 is not resolved",
      "safety": {
        "network": false,
        "writes": true,
        "writes_sot": false,
        "reads": true
      },
      "prerequisite": ["install_typst"]
    }
  ]
}
```

#### 3.4.2 paper 指定なし（プロジェクト全体）

```json
{
  "schema_version": "1.0",
  "project": { ... },
  "paper": null,
  "timestamp": "2026-01-05T12:34:56Z",
  "checks": [
    {
      "id": "project_structure",
      "name": "Project structure",
      "status": "ok",
      "message": "All required directories present"
    },
    {
      "id": "papers_found",
      "name": "Papers",
      "status": "ok",
      "message": "Found 3 papers",
      "details": {
        "papers": ["report", "thesis", "slides"]
      }
    }
  ],
  "actions": [ ... ]
}
```

#### 3.4.3 Check Status

| status | 意味 | ビルド可否 |
|--------|------|-----------|
| `"ok"` | 問題なし | 可能 |
| `"warning"` | 警告あり | 可能（Typst の warning 相当） |
| `"error"` | エラー | 不可能（Typst の error 相当） |

**重要な原則**：

- Typst の診断レベル（diagnostic level）に準拠
- `checks[].id` は必須（重複チェック識別用）
- `details` は任意の JSON

#### 3.4.4 Actions Schema

actions は次に実行可能なアクションを提示する。

**フィールド**：

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `id` | string | ✅ | アクションの一意識別子 |
| `command` | string | ✅ | 実行するコマンド |
| `description` | string | ✅ | 人間向け説明 |
| `enabled` | boolean | ✅ | 実行可能か |
| `disabled_reason` | string | ❌ | enabled=false の理由 |
| `safety` | object | ✅ | 副作用の宣言（network / reads / writes / writes_sot） |
| `prerequisite` | string[] \| null | ✅ | 依存する action ID |

**safety schema (v0.1)**：

- `network`: boolean（typstlab 自身のネットワーク通信のみ。typst/@preview や uv の内部通信は含まない）
- `reads`: boolean（プロジェクトルート配下のファイルを読みうるか）
- `writes`: boolean
- `writes_sot`: boolean（SOT を変更しうるか）

**network のスコープ**：

- `network` は **typstlab 自身が行うネットワーク通信のみ** を示す
- Typst の @preview パッケージダウンロード、uv の内部通信、Python スクリプトのネットワーク通信は制御対象外
- つまり `network: false` でも、外部ツールが内部でネットワーク通信を行う可能性がある
- エージェントは「typstlab の network ポリシーに準拠する操作」と理解すべき

**reads のスコープ**：

- `reads: true` は、「typstlab が **プロジェクトルート（typstlab.toml の存在するディレクトリ）配下のプロジェクトデータを読みうる** 操作である」ことを意味する
- 読み取り対象は **プロジェクトデータに限定**される
- **重要**：typst/uv の解決や検証のために、プロジェクトルート外（managed cache / system binary 等）をローカルに参照することはありうる。これは `reads` の分類対象外である
- **`reads: false` は「プロジェクトデータを読まない」を意味し、「ファイルシステムを一切読まない」を意味しない**

**symlink ポリシー（v0.1）**：

- typstlab が自前で走査する際は symlink を辿らない（`follow_links = false`）
- ただし、直接指定されたファイルパスが symlink の場合は読み取り自体は許可する
- その際、実体がプロジェクトルート外に解決される場合はエラーとする（例：`PROJECT_PATH_ESCAPE`）

**writes_sot のデフォルト原則**：

- `writes_sot` は **デフォルト true** として扱う
- `writes_sot: false` は「派生物（dist/, _generated/, bin/）/観測物（.typstlab/kb/）/観測キャッシュ（state.json）のみを書き換えることが仕様上保証できる操作」に限る
- false を付けていい条件（v0.1）：
  - **条件A**: 出力先が派生物/観測物/観測キャッシュに限定され、かつ SOT（typstlab.toml, paper.toml, main.typ, library.bib, sources.lock, layouts/, pyproject.toml, uv.lock）に触らないことが仕様上保証できる
  - **条件B**: コマンド自身が SOT への書き込みパスを持たない（例：reads=false, writes=false の場合）
- この原則により、エージェントは保守的に動作でき、事故を防げる

**enabled と disabled_reason**：

- **enabled: true**: 現在実行可能
- **enabled: false**: 実行不可能、disabled_reason で理由を説明
  - 例: `"network policy is 'never'"`
  - 例: `"Typst 0.13.1 is not resolved"`
- **重要**: disabled_reason は「なぜ今できないか（状態）」を説明する
  - prerequisite は「どうすればよいか（推奨アクション）」を示す

**prerequisite の定義**：

- prerequisite は「状態条件を満たすための推奨アクション」であり、実行履歴ではない
- enabled 判定は checks の状態から決定される
- prerequisite はあくまで「この action を実行する前に、これらの action を完了させることを推奨」という宣言

**破壊的操作（destructive）の扱い**：

- v0.1 では `destructive` フィールドは追加しない（シンプルさ優先）
- 代わりに、破壊的操作（削除、クリア等）は **description で明示** する
  - 良い例: `"Clear Typst docs (destructive)"`, `"Remove generated files"`
  - エージェントは description から破壊性を推論できる
- 破壊的操作の典型例：
  - `typst docs clear`: 観測物の削除（診断情報の喪失）
  - `clean`: 派生物の削除（再生成コスト）
- v0.2 以降で `destructive: boolean` の追加を検討

**重要な原則**：

- **actions は常に列挙**：network=never でも消さず、enabled=false で提示
- **自動実行判断はエージェント側**：automated フィールドなし、safety のみ提供
- エージェントの思想を尊重（Conservative なエージェントは network=true を避ける等）

### 3.5 doctor --json

ツールチェーンと環境の健全性を診断する。

```json
{
  "schema_version": "1.0",
  "project": {
    "name": "my-research",
    "root": "/Users/alice/projects/my-research"
  },
  "timestamp": "2026-01-05T12:34:56Z",
  "checks": [
    {
      "id": "typst_available",
      "name": "Typst toolchain",
      "status": "ok",
      "message": "Typst 0.13.1 resolved from managed cache",
      "details": {
        "required": "0.13.1",
        "resolved": "0.13.1",
        "source": "managed"
      }
    },
    {
      "id": "uv_available",
      "name": "uv toolchain",
      "status": "error",
      "message": "uv not found",
      "details": {
        "required": true
      }
    },
    {
      "id": "docs_integrity",
      "name": "Typst docs",
      "status": "warning",
      "message": "Docs are for 0.12.0, project requires 0.13.1",
      "details": {
        "docs_version": "0.12.0",
        "required_version": "0.13.1"
      }
    },
    {
      "id": "project_structure",
      "name": "Project structure",
      "status": "ok",
      "message": "All required directories present"
    }
  ],
  "actions": [
    {
      "id": "link_uv",
      "command": "typstlab link uv",
      "description": "Link uv from system",
      "enabled": true,
      "safety": {
        "network": false,
        "writes": true,
        "writes_sot": false,
        "reads": true
      },
      "prerequisite": null
    },
    {
      "id": "sync_docs",
      "command": "typstlab typst docs sync",
      "description": "Update docs to match Typst version",
      "enabled": true,
      "safety": {
        "network": true,
        "writes": true,
        "writes_sot": false,
        "reads": true
      },
      "prerequisite": null
    }
  ]
}
```

**doctor の役割**：

- ツールの可用性チェック（Typst, uv）
- ツール周りの整合性チェック（bin/, docs）
- プロジェクト構造の検証

**status との違い**：

- **doctor**: ツールチェーンと環境（project 視点）
- **status**: ビルド可能性（paper 視点）

**重要な原則**：

- **actions schema は status/doctor/MCP で完全に統一**
- 全ての actions は `enabled`, `disabled_reason` (optional), `safety`, `prerequisite` を持つ
- JSON schema は1つ（実装の分岐を防ぐため）

---

## 4. Layouts System

### 4.1 Concept

layouts システムは、paper.toml から自動生成される typst ファイル（`_generated/`）のテンプレートを管理する。

**目的**：

- 再現性の担保（レイアウトを明示的に指定）
- カスタマイズの自由（project-level でオーバーライド）
- フレームワークとしての一貫性

### 4.2 Structure

```
layouts/                      # project-level レイアウト定義
  default/                    # デフォルトレイアウト
    meta.tmp.typ              # paper メタ情報テンプレート（{{ }} 使用）
    header.typ                # ドキュメント設定・ヘッダー（静的）
    refs.tmp.typ              # 参考文献設定テンプレート（{{ }} 使用）
  ieee/                       # IEEE スタイル
    meta.tmp.typ
    header.typ
    refs.tmp.typ
  minimal/                    # ミニマル
    meta.tmp.typ
    refs.tmp.typ
    # header.typ なし（不要なものは省略可能）
```

**ファイル拡張子の規約**：

- `.tmp.typ`: テンプレートファイル（`{{ }}` プレースホルダーを含む）
  - `meta.tmp.typ`, `refs.tmp.typ`
  - IDE でも有効な Typst 構文として認識される
  - `generate` コマンドでプレースホルダーが置換され、`_generated/*.typ` が生成される
- `.typ`: 静的ファイルまたは生成されたファイル（プレースホルダーなし）
  - `header.typ` (静的、そのままコピー)
  - `_generated/meta.typ` (生成物)
  - `_generated/refs.typ` (生成物)

### 4.3 Resolution Order (v0.1)

v0.1 では以下の順序で解決：

1. `layouts/<layout-name>/<file>` (project-level)
2. builtin layouts

**将来（v0.2+）**：

1. `papers/<id>/layouts/<file>` (paper-level、最優先)
2. `layouts/<layout-name>/<file>` (project-level)
3. builtin layouts

### 4.4 Layout Files

#### 4.4.1 meta.tmp.typ → meta.typ

paper.toml から動的生成される。paper のメタ情報を Typst の dict として定義。

**テンプレート例** (`layouts/default/meta.tmp.typ`):

```typst
// AUTO-GENERATED by typstlab from paper.toml
// DO NOT EDIT - Changes will be overwritten

#let paper_meta = (
  id: "{{ ID }}",
  title: "{{ TITLE }}",
  authors: (
    {{ AUTHORS }}  // 動的に生成
  ),
  date: datetime(
    year: {{ YEAR }},
    month: {{ MONTH }},
    day: {{ DAY }}
  ),
  language: "{{ LANGUAGE }}",
)
```

**生成例** (`papers/report/_generated/meta.typ`):

```typst
// AUTO-GENERATED by typstlab from paper.toml
// DO NOT EDIT

#let paper_meta = (
  id: "report",
  title: "My Research Report",
  authors: (
    (name: "Alice", email: "alice@example.com", affiliation: "University"),
    (name: "Bob", email: "bob@example.com", affiliation: "Company"),
  ),
  date: datetime(year: 2026, month: 1, day: 5),
  language: "en",
)
```

#### 4.4.2 header.typ

ドキュメント設定とヘッダーを定義。静的コピーまたは軽い置換。

**例** (`layouts/default/header.typ`):

```typst
#import "_generated/meta.typ": paper_meta

#set document(
  title: paper_meta.title,
  author: paper_meta.authors.map(a => a.name),
  date: paper_meta.date,
)

#set page(
  header: align(right, paper_meta.title),
  numbering: "1",
)

#set text(lang: paper_meta.language)
```

#### 4.4.3 refs.tmp.typ → refs.typ

参考文献設定。paper.toml の [refs].sets から bibliography 呼び出しを生成する。

**テンプレート例** (`layouts/default/refs.tmp.typ`):

```typst
// AUTO-GENERATED by typstlab from paper.toml

{{ BIBLIOGRAPHY }}
```

**生成例** (`papers/report/_generated/refs.typ`):

```typst
// AUTO-GENERATED by typstlab from paper.toml

#bibliography("../../refs/sets/core/library.bib")
#bibliography("../../refs/sets/report-2026q1/library.bib")
```

**refs が空の場合**:

```typst
// AUTO-GENERATED by typstlab from paper.toml
// No bibliography specified
```

### 4.5 Generation Flow

```
1. paper.toml を読む
   ↓
2. layout 解決
   - paper.toml の [layout] theme を取得
   - layouts/<theme>/ を探す
   - なければ builtin を使う
   ↓
3. _generated/meta.typ を生成
   - layouts/<theme>/meta.typ をテンプレートとして使用
   - paper.toml の値で {{ PLACEHOLDERS }} を置換
   ↓
4. _generated/header.typ をコピー
   - layouts/<theme>/header.typ をそのままコピー
   ↓
5. _generated/refs.typ を生成
   - layouts/<theme>/refs.typ をテンプレートとして使用
   - paper.toml の [refs].sets から bibliography 呼び出し列を生成し {{ BIBLIOGRAPHY }} を置換
   ↓
6. ビルド時に typst compile を実行
```

### 4.6 Built-in Layouts

typstlab が提供する組み込みレイアウト（v0.1）：

| Layout | 説明 | ファイル |
|--------|------|---------|
| `default` | シンプルな学術論文 | meta.typ, header.typ, refs.typ |
| `minimal` | 最小限（header なし） | meta.typ, refs.typ |

### 4.7 _generated/ の管理原則

`_generated/` は常に **完全な派生物** であり、厳密なルールで管理される。

**原則**:

- **常に派生物**：`_generated/` は paper.toml + layouts から生成される
- **gitignore 必須**：git commit してはいけない
- **手編集禁止**：ユーザーが直接編集してはいけない
- **唯一の生成者**：`typstlab sync` のみが生成・更新できる
- **破棄可能**：削除しても `sync` で再生成できる

**CI での扱い**:

```yaml
# CI の例
steps:
  - checkout
  - run: typstlab setup             # 環境構築（ツールInstall + 生成）
  - run: typstlab build --paper report
```

**エラー検知**:

- _generated/ が古い（paper.toml より古い mtime）→ 警告
- _generated/ が手編集されている → 検出は困難だが、常に再生成で上書き

**重要な理由**:

- 再現性の担保：_generated/ を commit すると、どちらが正か分からなくなる
- エージェント操作性：常に最新の状態を保証

---

## 5. Command Specifications

### 5.1 Exit Code Policy

| コマンド種別 | 成功 | 失敗 | 理由 |
|-------------|------|------|------|
| 状態取得系（status, doctor） | exit 0 | exit 0 (JSON 内でエラー) | エージェント操作性 |
| 実行系（build, gen, new, setup, sync） | exit 0 | exit 1 | CI/CD, 人間の利用 |

**JSON 出力の I/O 規約**：

- `--json` 時は stdout に JSON のみを出力し、stderr には人間向けメッセージを出さない（必要なら `--verbose` 等で制御）
- これにより、エージェントは stdout を安全にパースでき、stderr を監視する必要がない

### 5.2 Lifecycle Commands

#### 5.2.1 `typstlab new <project-name>`

新しい typstlab プロジェクトディレクトリを作成する。
デフォルトでは Paper は作成されない（空のプロジェクトのみ）。

**Usage**:

```bash
typstlab new my-research
typstlab new my-research --paper report
```

**Options**:

- `--paper <name>`: プロジェクト作成後に指定した名前で Paper を生成する (Shortcut for `gen paper`)

**動作**:

1. `<project-name>/` ディレクトリを作成
2. `typstlab.toml` を生成
3. 必須ディレクトリ構造を作成
4. (`--paper` 指定時) `typstlab gen paper <name>` 相当を実行

**Safety classification**:

- `network`: false
- `writes_sot`: true

#### 5.2.2 `typstlab init [path]`

カレントディレクトリ（または指定パス）をプロジェクトとして初期化する。

**Usage**:

```bash
typstlab init
typstlab init .
typstlab init --paper report
```

**Options**:

- `--paper <name>`: 初期化後に指定した名前で Paper を生成する

**動作**:

1. `new` と同様だが、ディレクトリ作成を行わず、既存ディレクトリ内に展開する
2. 既に `typstlab.toml` がある場合はエラー
3. (`--paper` 指定時) `typstlab gen paper <name>` 相当を実行

**Safety classification**:

- `network`: false
- `writes_sot`: true

#### 5.2.3 `typstlab paper list`

プロジェクト内のすべての paper を一覧表示する。

**Usage**:

```bash
typstlab paper list
typstlab paper list --json
```

**Options**:

- `--json`: JSON 形式で出力

**動作**:

1. プロジェクトルートを検索
2. `papers/` ディレクトリをスキャン
3. 各 paper の情報を表示（ID / Title / Language / Date）

**Safety classification**:

- `network`: false
- `reads`: true (papers/, paper.toml)
- `writes`: false

### 5.3 Scaffolding Commands (`gen`)

#### 5.3.1 `typstlab gen paper <id>`

プロジェクト内に新しい Paper を生成する。

**Usage**:

```bash
typstlab gen paper report
typstlab gen paper thesis --layout ieee --title "My Thesis"
```

**Options**:

- `--layout <name>`: テーマを指定 (default: project default)
- `--title <title>`: タイトルを指定

**動作**:

1. `papers/<id>/` を作成
2. `paper.toml`, `main.typ` を生成
3. `_generated/` 等の初期生成 (`sync` 相当の処理を含む)

**Safety classification**:

- `network`: false
- `writes_sot`: true

#### 5.3.2 `typstlab gen layout <name>`

カスタムレイアウトのテンプレートを生成する。

**Usage**:

```bash
typstlab gen layout my-layout
```

**動作**:

1. `layouts/<name>/` を作成
2. `meta.tmp.typ`, `header.typ`, `refs.tmp.typ` を生成

**Safety classification**:

- `network`: false
- `writes`: true
- `writes_sot`: true（layouts/ は SOT）

#### 5.3.3 `typstlab gen lib <name>`

> **Note**: v0.2以降で実装予定。v0.1では未実装（"Not implemented" エラーまたは警告を表示）。

新しいライブラリパッケージの構成を生成する。

### 5.4 Operational Commands

#### 5.4.1 `typstlab setup`

プロジェクトを利用可能な状態にする（Provisioning）。「Clone & Go」を実現するコマンド。

**Usage**:

```bash
cd my-project
typstlab setup
```

**動作**:

以下の処理を順次実行するエイリアスとして振る舞う：

1.  `typstlab doctor --fix` 相当 (Network)
    *   Typst ツールチェーンの解決・インストール (`typst install`)
    *   Docs の欠損確認・同期 (`typst docs sync`)
    *   uv (Python) の解決 (`link uv`)
2.  `typstlab sync --all`

**Safety classification**:

- `network`: true
- `writes`: true
- `writes_sot`: false (原則)

#### 5.4.2 `typstlab sync [flags]`

プロジェクト状態の整合性を確保する（Consistency）。アーティファクトの生成・更新を担うハブコマンド。

**Usage**:

```bash
typstlab sync                  # Default: Local artifacts only
typstlab sync --paper report   # Specific paper only
typstlab sync --docs           # Include docs sync (Network)
typstlab sync --tools          # Include tool installation (Network)
typstlab sync --all            # Everything (Setup equivalent)
```

**動作**:

1.  (Default) `_generated/` (Layout templates) の再生成
2.  (Default) `bin/` shims の再生成
3.  (--docs) `typstlab typst docs sync` を実行
4.  (--tools) `typstlab typst install` (Smart resolution) を実行

**Safety classification**:

- `network`: false (default) / true (with flags)
- `writes`: true
- `writes_sot`: false

#### 5.4.3 `typstlab generate [--paper <id>]`

_generated/ を生成・更新する（ビルドはしない）。

**Usage**:

```bash
typstlab generate --paper report   # 特定 paper
typstlab generate                  # 全 paper
```

**動作**:

1. paper.toml を読む
2. layout を解決
3. _generated/ を生成
4. state.json は更新しない（ビルドしていないので）

**Safety classification**:

- `network`: false
- `reads`: true
- `writes`: true（papers/*/_generated/ を更新）
- `writes_sot`: false

### 5.5 Execution Commands

#### 5.5.1 `typstlab build [target]`

Paper をビルドする。

**Usage**:

```bash
typstlab build                 # Build all papers (Parallel)
typstlab build -p report       # Build specific paper
typstlab build papers/report   # Build from path context
```

**Options**:

- `-p, --paper <id>`: ID指定
- `--full` / `--force`: ビルド前に `sync` (local) を実行してアーティファクトを全再生成

**動作**:

1.  ターゲットの特定（指定なしなら全Paper）
2.  (Optional) `sync` 実行
3.  並列ビルド実行 (`typst compile ...`)
4.  結果レポート

**Safety classification**:

- `network`: false
- `writes`: true (dist/)

#### 5.5.2 `typstlab watch`

> **Note**: v0.2で実装予定。v0.1では未実装。

指定した paper の変更を監視して自動ビルドする。

**Usage**:

```bash
typstlab watch --paper report
```

**動作**:

1. 依存ファイルを監視（main.typ, sections/, assets/, refs/, figures/）
2. paper.toml の変更も監視
3. 変更検知 → debounce (500ms) → build
4. Typst の incremental compilation に任せる

**Safety classification**:

- `network`: false
- `reads`: true
- `writes`: true（build と同等）
- `writes_sot`: false

### 5.6 Status & Diagnosis

#### 5.6.1 `typstlab status [target]`

プロジェクトまたは Paper の状態を表示する。

**Usage**:

```bash
typstlab status                # Project overall status
typstlab status -p report      # Paper status
typstlab status --json         # JSON output
```

**動作**:

- プロジェクト構成、Paper一覧、健全性チェック結果を表示
- `typstlab paper list` (旧) の機能もここに統合

#### 5.6.2 `typstlab doctor`

環境の詳細診断を行う。デバッグ用。

**Usage**:

```bash
typstlab doctor
typstlab doctor --json
```

### 5.7 Toolchain Utilities (`typst`)

#### 5.7.1 `typstlab typst install [version]`

Typst をスマートにインストール・解決する。

**Usage**:

```bash
typstlab typst install         # typstlab.toml のバージョンを解決
typstlab typst install 0.13.0  # 指定バージョン
typstlab typst install --offline
```

**動作 (Smart Resolution)**:

1.  **System Check**: PATH上の `typst` が要求バージョンと一致？ -> Link (Symlink作成)
2.  **Cache Check**: Managedキャッシュ(`~/.cache/...`)にある？ -> Use Cache
3.  **Download**: 上記になければGitHubからダウンロード (Network)

**Safety classification**:

- `network`: true (unless --offline)
- `writes`: true (cache, bin/)

#### 5.7.2 `typstlab typst docs sync`

ドキュメントの同期。

### 5.8 Experimental / Advanced

#### 5.8.1 `typstlab mcp`

MCP (Model Context Protocol) サーバーを起動する。

**Usage**:

```bash
typstlab mcp stdio
```

#### 5.8.2 `typstlab lsp`

> **Note**: v0.1では試験的実装、またはプレースホルダー。

LSP (Language Server Protocol) サーバーを起動する。

**Usage**:

```bash
typstlab lsp stdio
```

**動作**:

- `tinymist` 等のLSPサーバー機能をラップして起動、または `Stdio` 経由でエディタと通信する。
- v0.1 では "Not implemented" の警告、または最小限の起動シーケンスのみ実装。


### 5.7 Typst Commands

#### 5.7.1 `typstlab typst link [-f]`

system の typst を探索して bin/typst shim を生成する。

**Usage**:

```bash
typstlab typst link       # 初回のみ実行
typstlab typst link -f    # 強制的に再探索
```

**動作**:

1. `which typst` で探索
2. `typst --version` でバージョン確認
3. typstlab.toml の要求バージョンと完全一致するか確認
4. 一致すれば bin/typst shim を生成
5. state.json を更新

**Safety classification (v0.1)**：

- `network`: false
- `reads`: true
- `writes`: true（bin/typst と state.json）
- `writes_sot`: false

**Exit code**: 成功 0, 失敗 1

#### 5.7.2 `typstlab typst install <version>`

指定バージョンの typst を managed にインストールする。

**Usage**:

```bash
typstlab typst install 0.13.1
```

**動作**:

1. GitHub Releases API から release metadata を取得
   - API endpoint: `https://api.github.com/repos/typst/typst/releases/tags/v{version}`
2. OS/arch に一致する asset を選択
   - asset name に `{os}` と `{arch}` を含むものを探す
   - 例: `typst-x86_64-apple-darwin.tar.gz`
3. asset をダウンロードして managed cache（6.1.2参照）に展開する
   - 展開物から typst バイナリを抽出し、`{managed_cache_dir}/{version}/typst`（Windows は `typst.exe`）として配置する
   - サブディレクトリは作らない（`{version}/` 直下に実行ファイルを置く）
4. `typst --version` で最終検証（バージョン一致を確認）
5. state.json を更新
6. cargo fallback（リリースから取得できない場合）
   - `cargo install typst-cli --version {version}`
   - cargo が利用可能かチェック
   - インストール後に `typst --version` で検証

**重要な原則**:

- URL は固定せず、Release API から動的に取得
- ダウンロード後は必ず `typst --version` で検証
- cargo fallback は最終手段（時間がかかる）

**Safety classification (v0.1)**：

- `network`: true
- `reads`: true
- `writes`: true（managed cache と state.json を更新）
- `writes_sot`: false（プロジェクトの SOT は変更しない）

**Exit code**: 成功 0, 失敗 1

#### 5.7.3 `typstlab typst version [--json]`

要求バージョンと解決バージョンを表示する。

**Usage**:

```bash
typstlab typst version
typstlab typst version --json
```

**出力例**（macOS）:

```json
{
  "required": "0.13.1",
  "resolved": "0.13.1",
  "source": "managed",
  "path": "/Users/alice/Library/Caches/typstlab/typst/0.13.1/typst"
}
```

**注**：`path` は OS に依存する（6.1.2 参照）。

**Exit code**: 常に 0

#### 5.7.4 `typstlab typst versions [--json]`

インストール済みの Typst バージョン一覧を表示する。

**Usage**:

```bash
typstlab typst versions
typstlab typst versions --json
```

**Output example**:

```plaintext
Installed Typst versions:
  • 0.12.0 (managed)
  • 0.13.0 (managed)
  • 0.13.1 (managed)
```

**Safety classification (v0.1)**：

- `network`: false
- `reads`: true
- `writes`: false

**Exit code**: 常に 0

#### 5.7.5 `typstlab typst exec -- <args>`

解決済み typst を実行する。bin/typst shim から呼ばれる。

**Usage**:

```bash
typstlab typst exec -- compile main.typ
# または bin/typst 経由
bin/typst compile main.typ
```

**動作**:

1. state.json から resolved_path を読む
2. プロジェクトルート検証（typstlab.toml が存在するか）
3. typst を実行
4. （将来）監査ログ記録

**Exit code**: typst の exit code をそのまま返す

#### 5.7.5 `typstlab typst docs sync`

Typst docs (MD) を取得・更新する。

**Usage**:

```bash
typstlab typst docs sync
```

**動作**:

1. 要求バージョンを取得
2. Typst Community dev-builds から docs.json を取得
   - データソース: `https://github.com/typst-community/dev-builds/releases/download/docs-vX.Y.Z/docs.json`
   - docs.json には HTML content + 構造化メタデータが含まれる
   - 例: `https://github.com/typst-community/dev-builds/releases/download/docs-v0.12.0/docs.json`
3. HTML → Markdown 変換（2-stage pipeline）
   - **Stage 1**: HTML → mdast (html_to_mdast.rs)
     - html5ever でHTMLをパース
     - mdast (Markdown Abstract Syntax Tree) ノードを直接構成
     - サポート要素: heading, paragraph, link, list, table, blockquote, code, emphasis, strong
     - Typst syntax spans (`typ-*` classes) は inline code にフラット化
   - **Stage 2**: mdast → Markdown (mdast_util_to_markdown + custom table renderer)
     - **Base**: CommonMark 100% 準拠保証 (markdown-rs ecosystem, 2300+ tests)
     - **GFM Tables**: カスタムレンダラー (mdast_util_to_markdown v0.0.2 は Table 非サポート)
       - 形式: GFM pipe table (`| Header | Header |`)
       - 機能: カラムアライメント (`:---`, `:---:`, `---:`)、インライン整形、パイプエスケープ
       - 参照実装: pulldown-cmark (GitHub公式GFMパーサー)
     - alpha dependency (v0.0.2) だがバージョン固定 + 包括的テストでリスク軽減
     - **Fallback 戦略**:
       1. Table ノード検出時 → カスタム GFM レンダラー使用
       2. その他のエラー → plain text fallback（dual spec 回避）
4. YAML frontmatter 生成
   - `serde_yaml`で構造化データからYAML生成
   - `title`フィールド: 常に含まれる
   - `description`フィールド: 存在する場合のみ含まれる
   - 本文のh1見出しは削除（frontmatterのtitleで代替）
5. 階層的ディレクトリ構造で`.typstlab/kb/typst/docs/`に保存
   - route → filepath マッピング（path traversal防止）
   - **リンク形式**: v0.2.0以降、ディレクトリに`index.md`を使用しない新形式を採用
     - 変換ルール（`rewrite_docs_link()`）:
       - Root: `/DOCS-BASE/` → `../index.md`（ルートのみ例外）
       - Directories: `/DOCS-BASE/tutorial/` → `../tutorial.md`
       - Files: `/DOCS-BASE/tutorial/writing` → `../tutorial/writing.md`
       - Fragments: `/DOCS-BASE/tutorial/#section` → `../tutorial.md#section`（保持）
       - Query strings: `/DOCS-BASE/api?v=1` → `../api.md?v=1`（保持）
       - External URLs: `https://...` → 変更なし
       - Other schemes: `mailto:`, `tel:`, `#...` → 変更なし
     - セキュリティ: パストラバーサル (`..`) を含むパスは変換しない
     - ファイル構造例:

       ```plaintext
       .typstlab/kb/typst/docs/
         index.md                    # ルート
         tutorial.md                 # トップレベルディレクトリ
         tutorial/
           writing.md                # ネストされたページ
           formatting.md
         reference/
           styling.md
       ```

     - **破壊的変更**: 既存の`tutorial/index.md`などは削除され、`tutorial.md`として再生成
6. state.json を更新

**アーキテクチャ設計判断**:

- **ロジックの分散**: `html_to_mdast.rs`で要素ハンドリングが複数関数に分散
  - `handle_element_start()`: 要素判定とルーティング（~250行）
  - 個別ビルダー関数: `build_heading()`, `build_link()`, `build_list()` など（各10-50行）
  - **理由**: Single Responsibility Principle、テスト容易性、AGENTS.md (40行/関数制限) 準拠
- **Type safety**: Rust の型システムで正確性保証（`Path`/`PathBuf` for cross-platform、mdast nodes for structure）
- **Extensibility**: mdast は plugin 拡張可能（future: syntax highlighting restoration、linting、optimization）

**v0.1 の最低限 contract**：

- docs は optional（存在しない場合は `status`/`doctor` で warning とし、actions に `sync_docs` を提示）
- `source: "official"` の定義：typst-community/dev-builds の docs.json（機械生成構造化データ）
- エージェントが Typst の型情報・関数情報を LLM-friendly Markdown で取得できることが目的
  - CommonMark 準拠で AI parsing 容易性保証
  - YAML frontmatter で構造化メタデータ提供

**Safety classification (v0.1)**：

- `network`: true
- `reads`: true
- `writes`: true（.typstlab/kb/typst/docs と state.json）
- `writes_sot`: false

**Exit code**: 成功 0, 失敗 1

#### 5.7.6 `typstlab typst docs clear`

Typst docs を削除する。

**Usage**:

```bash
typstlab typst docs clear
```

**Exit code**: 成功 0, 失敗 1

#### 5.7.7 `typstlab typst docs status [--json]`

docs の状態を表示する。

**Usage**:

```bash
typstlab typst docs status --json
```

**Exit code**: 常に 0

### 5.8 Link Command

> **Note**: v0.2で実装予定。v0.1では未実装。

#### 5.8.1 `typstlab link uv [-f]`

system の uv を探索して bin/uv shim を生成する。

**Usage**:

```bash
typstlab link uv
typstlab link uv -f
```

**動作**:

1. `which uv` で探索
2. `uv --version` でバージョン取得
3. bin/uv shim を生成
4. state.json を更新

**Safety classification (v0.1)**：

- `network`: false
- `reads`: true
- `writes`: true（bin/uv と state.json）
- `writes_sot`: false

**Exit code**: 成功 0, 失敗 1

#### 5.8.2 `typstlab uv exec -- <args>`

解決済み uv を実行する。bin/uv shim から呼ばれる。

**Usage**:

```bash
typstlab uv exec -- pip install numpy
# または bin/uv 経由
bin/uv pip install numpy
```

**前提条件**:

- `typstlab link uv` により uv が解決済みであること
- 未解決の場合は `UV_NOT_RESOLVED` エラーで失敗し、action `link_uv` を提示

**動作**:

1. state.json から resolved_path を読む
2. resolved_path が存在しない場合はエラー（解決は link の責務）
3. プロジェクトルート検証（typstlab.toml が存在するか）
4. uv を実行

**Safety classification (v0.1)**：

- `writes`: true（uv 実行は書き込みを伴う可能性があるため）
- `writes_sot`: **true**（常に保守的に扱う）
  - v0.1 では uv exec は常に writes_sot: true とする
  - uv のコマンドは pyproject.toml / uv.lock（SOT）を変更しうるため
  - 引数 allowlist によって false を付けることも可能だが、コストの割に価値が薄いため v0.1 では見送り

**Exit code**: uv の exit code をそのまま返す（未解決時は exit 1）

### 5.9 Refs Commands

> **Note**: v0.2で実装予定。v0.1では未実装。

refs コマンドは、参考文献の取得と履歴管理を担当する。
論文用途を尊重し、**アクセス日時を first-class** に扱う。

#### 5.9.1 `typstlab refs fetch --set <set-id> --doi <doi> | --url <url>`

DOI または URL から BibTeX を新規取得して `refs/sets/<set-id>/library.bib` に追加する。

**Usage**:

```bash
typstlab refs fetch --set core --doi 10.1234/example
typstlab refs fetch --set report-2026q1 --url https://arxiv.org/abs/2301.00000
```

**Options**:

- `--set <set-id>`: 追加先の refs set（必須）
- `--doi <doi>`: DOI から取得
- `--url <url>`: URL から取得

**動作**:

1. DOI または URL から BibTeX を取得
   - DOI: CrossRef API など
   - URL: arXiv, 論文サイト等（可能な範囲）
2. `refs/sets/<set-id>/library.bib` に追加
3. `refs/sets/<set-id>/sources.lock` に取得履歴を記録
   - source（DOI または URL）
   - fetched_at（取得日時）
   - hash/etag（optional）

**重要な原則**:

- 取得日時が**情報源の信頼性の証明**となる
- 「このタイミングで絶対に正しい」ことを保証する

**Safety classification (v0.1)**：

- `network`: true
- `reads`: true
- `writes`: true
- `writes_sot`: true（library.bib / sources.lock は SOT）

**Exit code**: 成功 0, 失敗 1

#### 5.9.2 `typstlab refs check [--set <set-id>] [--paper <paper-id>]`

既存の refs の整合性を検証する（ネットワーク不要）。

**Usage**:

```bash
typstlab refs check
typstlab refs check --set core
typstlab refs check --paper report
```

**動作**:

1. 対象の refs set を解決する
   - `--set`: 指定 set のみ
   - `--paper`: `papers/<paper-id>/paper.toml` の `[refs].sets` のみ
   - 指定なし: 全 set
2. 各 set の `library.bib` を読む
3. 各 set の `sources.lock` と突き合わせ
4. 不整合を検出
   - set の片方ファイルのみ存在（library.bib / sources.lock）
   - library.bib にあるが sources.lock にない
   - sources.lock にあるが library.bib にない
   - cite されているが、参照対象 set 群に該当 key が存在しない（main.typ を parse）
   - 同一 paper が参照する set 間で key が衝突（error）
5. 結果を報告

**Safety classification (v0.1)**：

- `network`: false
- `reads`: true
- `writes`: false
- `writes_sot`: false

**Exit code**: 成功 0, 失敗 1

#### 5.9.3 `typstlab refs touch --set <set-id> [--key <key>] [--all]`

既存の refs に今日の日付で再アクセスして記録を更新する。

**Usage**:

```bash
typstlab refs touch --set core --key smith2020     # 特定エントリ
typstlab refs touch --set core --all               # 全エントリ
```

**動作（v0.1）**:

1. `refs/sets/<set-id>/sources.lock` から対象エントリを取得
2. source（DOI/URL）に再アクセス
3. `refs/sets/<set-id>/sources.lock` の last_accessed を今日の日付に更新
4. **library.bib は更新しない**（v0.1 の安全性担保）

**用途**:

- 論文執筆中に「この情報源を確認した」記録を残す
- 古いエントリの有効性を再確認

**重要な原則**:

- v0.1 では **アクセス記録のみ** を目的とする
- BibTeX メタデータの更新は別コマンド（v0.2 以降で `typstlab refs refresh` を検討）
- 引用が勝手に変わる事故を防ぐため、library.bib は保護
- **`refs touch` は内容の正しさを保証しない**
  - あくまで「人間がその時点でアクセスした」という事実のみを記録する
  - source の内容が変わっていても検出しない

**Safety**:

- `network`: true（source に再アクセスするため）
- `writes`: true（sources.lock を更新）
- `reads`: true

**Safety classification (v0.1)**：

- `network`: true
- `reads`: true
- `writes`: true
- `writes_sot`: true（sources.lock は SOT）

**Network Policy**:

- `network = "never"` 時はエラー
- status/doctor の actions では `enabled: false, disabled_reason: "network policy is 'never'"` で表示

**Exit code**: 成功 0, 失敗 1

#### 5.9.4 sources.lock の役割

`refs/sets/<set-id>/sources.lock` は refs set 単位で、参考文献取得・アクセスの履歴を記録する。

**形式**:

```json
{
  "schema_version": "1.0",
  "entries": [
    {
      "key": "smith2020",
      "source": "doi:10.1234/example",
      "fetched_at": "2026-01-05T12:00:00Z",
      "last_accessed": "2026-01-10T09:30:00Z",
      "hash": "sha256:...",
      "metadata": {
        "title": "Example Paper",
        "authors": ["Smith, J."],
        "year": 2020
      }
    }
  ]
}
```

**重要フィールド**:

- `fetched_at`: 最初に取得した日時（変更不可）
- `last_accessed`: 最後にアクセスした日時（touch で更新）
- `hash`: optional、メタデータの検証用

### 5.10 MCP Command

#### 5.10.1 `typstlab mcp serve [--offline]`

MCP サーバを起動する。

**Usage**:

```bash
typstlab mcp serve
typstlab mcp serve --offline
```

**公開ツールセット（v0.1）**  
モードに応じて *list_tools* に載せるツールを切り替える。公開していないツールはクライアントに見えない。

| Mode | 公開ツール |
| --- | --- |
| online | `rules_browse`, `rules_search`, `rules_page`, `rules_list`, `docs_browse`, `docs_search`, `cmd_generate`, `cmd_build`, `cmd_status`, `cmd_typst_docs_status` |
| offline | 上記から `cmd_generate`, `cmd_build` を除外 |

**listChanged 通知**: online/offline の切替時にツール一覧が変わるため、`listChanged` を送出する。  
**Exit code**: 中断まで実行

#### 5.10.2 MCP Tools Architecture

typstlab MCP サーバーは、AI エージェントが目的を明確に区別できるよう、ツール名にプレフィックス規則を採用する。

| Prefix | Category | Description | Safety |
| --- | --- | --- | --- |
| `cmd_*` | **Command** | CLI コマンド (`typstlab <cmd>`) のラッパー。副作用（ビルド、生成、状態変更）を伴う実行系ツール。 | `writes: true` (often) |
| `docs_*` | **Documentation** | ドキュメントの閲覧・検索・構造探索を行うツール。 | `reads: true`, `writes: false` |
| `feat_*` | **Feature** | AI エージェント向けの便利機能・複合機能。CLI には直接対応しない高レベルなタスク（例：画像生成して確認）を提供する。 | Varies |
| `rules_*` | **Rules** | プロジェクトの規約 (`rules/`) や設定を参照するツール。 | `reads: true`, `writes: false` |

#### 5.10.3 Provided MCP Tools

##### Rules Tools (`rules_*`)

- `rules_browse`：ディレクトリ探索
- `rules_search`：全文検索
- `rules_page`：部分読み出し（offset/limit）
- `rules_list`：全ルール列挙（サイズ・件数制限あり）

###### rules_browse

List files/directories in rules paths.
(Replaces previous `rules_list`. Use `read_resource` to read content.)

**Input Schema**:

```json
{
  "path": "rules"  // Path relative to project root. Must allow access to `rules/` or `papers/*/rules/`
}
```

**Output Schema**:

```json
{
  "items": [
    {
      "name": "paper",
      "type": "directory",
      "path": "rules/paper"
    },
    {
      "name": "guidelines.md",
      "type": "file",
      "path": "rules/guidelines.md"
    }
  ]
}
```

**Path Resolution**:

- `path="rules"` → `rules/`
- `path="papers/id/rules"` → `papers/id/rules/`

**Constraints**:

- Only .md files
- No hidden files (starting with .)
- No symlinks or files resolving outside project root

**Safety classification (v0.1)**:

- `network`: false
- `reads`: true
- `writes`: false
- `writes_sot`: false

###### rules_search

Context-aware full-text search across rules.
Can search within a specific paper's rules, the project root rules, or both.

**Input Schema**:

```json
{
  "query": "citation format",
  "paper_id": "paper1",    // optional. If specified, searches papers/<id>/rules/
  "include_root": true,    // default true. If true, also searches rules/
  "page": 1                // optional, default 1. 1-based page number.
}
```

**Output Schema**:

```json
{
  "matches": [
    {
      "path": "rules/paper/citations.md",
      "line": 42,
      "preview": "...use APA citation format for...",
      "origin": "root", // "root" | "paper"
      "uri": "typstlab://rules/paper/citations.md",
      "mtime": 1704450000
    }
  ],
  "truncated": false, // true if there are more results (next page exists)
  "missing": false
}
```

**Constraints**:

- Case-insensitive substring match
- Return 2 lines context before/after (preview)
- Max 50 matches per page (MAX_MATCHES)
- Pagination via `page` argument

**Safety classification (v0.1)**:

- `network`: false
- `reads`: true
- `writes`: false
- `writes_sot`: false

##### Command Tools (`cmd_*`)

###### cmd_generate

paper.toml からテンプレートファイル（`_generated/`）を生成する。
（CLI の `typstlab generate` に相当）

**Input Schema**:

```json
{
  "paper_id": "paper1"
}
```

**Output Schema**:

```json
{
  "status": "success",
  "message": "Successfully generated paper artifacts: paper1"
}
```

**Safety classification (v0.1)**:

- `network`: true (Template 解決で発生しうるため)
- `reads`: true
- `writes`: true (`_generated/` への書き込み)
- `writes_sot`: false (SOT は変更しない)

##### Documentation Tools (`docs_*`)

###### docs_browse

ドキュメントのディレクトリ構造を探索する。

**Input Schema**:

```json
{
  "path": "docs/reference" // optional, default root
}
```

**Output Schema**:

```json
{
  "items": [
    {
      "name": "syntax.md",
      "type": "file"
    },
    {
      "name": "visualize",
      "type": "directory"
    }
  ]
}
```

**Safety classification (v0.1)**:

- `network`: false
- `reads`: true
- `writes`: false
- `writes_sot`: false

###### docs_search

ドキュメント内を検索する。

**Input Schema**:

```json
{
  "query": "function definition",
  "page": 1 // optional, default 1
}
```

**Output Schema**:

```json
{
  "matches": [
    {
      "path": "docs/reference/syntax.md",
      "line": 15,
      "preview": "...",
      "uri": "typstlab://docs/reference/syntax.md",
      "mtime": 1704450000
    }
  ],
  "truncated": false, // true if there are more results
  "missing": false
}
```

**Safety classification (v0.1)**:

- `network`: false
- `reads`: true
- `writes`: false
- `writes_sot`: false

##### Feature Tools (`feat_*`)

###### feat_build_to_image_paper

Paper を指定フォーマット（PNG/SVG）でコンパイルし、確認用の成果物を生成する。
（エージェントが視覚的に出力確認を行うために使用）

**Input Schema**:

```json
{
  "paper_id": "paper1",
  "format": "png" // "png" | "svg", default "png"
}
```

**Output Schema**:

```json
{
  "images": [
    "dist/paper1/page1.png",
    "dist/paper1/page2.png"
  ]
}
```

**Safety classification (v0.1)**:

- `network`: true (パッケージダウンロードのため)
- `reads`: true
- `writes`: true (dist/ への書き込み)
- `writes_sot`: false (SOT は変更しない)

**Error Schema** (common to all tools):

```json
{
  "error": {
    "code": "PATH_ESCAPE" | "FILE_TOO_LARGE" | "NOT_FOUND" | "INVALID_INPUT",
    "message": "...",
    "details": {}
  }
}
```

---

#### 5.10.4 公開ポリシーと安全メタデータ

- list_tools には「現在利用可能なツールのみ」を掲載する。利用不可ツールを公開したままエラー返却で代替しない。  
- online/offline の切替で公開セットが変わる場合は `listChanged` を発火させる。  
- `open_world_hint` はネット依存ツールのみ `true`、`read_only_hint` は書き込みを行うツールで `false` とし、公開セットと整合させる。  
- 予約ツール（例: `feat_build_to_image_paper`）は v0.1 では非公開・非列挙。将来リリース項に記載する。

#### 5.10.5 スキーマ統一ポリシー

- search 系 (`rules_search`, `docs_search`) レスポンスは常に `{ "matches": [...], "truncated": bool, "missing": bool }`。対象ディレクトリが無い場合も同構造で `matches: []`, `truncated: false`, `missing: true`。  
- browse/list 系 (`rules_browse`, `rules_list`, `docs_browse`) は `{ "items" | "files": [...], "missing": bool, "truncated"?: bool }`。missing=true でも構造は固定。  
- `rules_page` は `{ "content": string, "offset": number, "limit": number, "total": number }`。  
- `rules_get` / `docs_get`:
  - Input: `{ "path": string, "page": number (optional, default 1) }`
  - Output: `{ "content": string, "truncated": boolean, "missing": boolean }`
  - `truncated=true` indicates next page exists.
  
- 上記スキーマは online/offline で変化させない。
- `path` フィールドは **MCP ベストプラクティス準拠で「返却された値がそのまま再入力に使える」こと**を保証する。  
  具体的には、`rules_*`/`docs_*` のレスポンスに含まれる `path` を再入力として渡した際に、同一対象へアクセスできること（E2E で検証する）。  
  返却形式（区切り文字や基準ディレクトリの表現）は実装詳細とせず、**E2E テストで互換性を担保**する。

#### 5.10.5.1 検索と取得の挙動（実装の約束事）

- **検索 (`rules_search` / `docs_search`)**  
  - ファイルを WalkDir で再帰走査し、`.md` ファイルを `std::fs::read_to_string` で全行読み込んで `line.to_lowercase().contains(query_lowercase)` の部分一致（単一サブストリング）を行う。複数語は `query` をそのまま使うため `foo bar` のような複合語を AND/OR で分割しない。  
  - 行ごとに `path` / `line` / `content` を JSON 化し、最大 `MAX_MATCHES` 件まで収集。`MAX_MATCHES_PER_FILE` 件を超えたらそのファイル内の追加抽出を止める。  
  - `matches`/`truncated`/`missing` のスキーマを必ず守り、`missing=true` は対象ディレクトリが存在しないとき。
  - **ページネーション**:
    - `page` 引数（デフォルト 1）を受け取る。
    - `MAX_MATCHES` (50) を1ページの固定サイズとし、`(page - 1) * MAX_MATCHES` 件をスキップして次の 50 件を取得する。
    - 上限（`MAX_MATCHES`）に達した場合、取得できた件数（最大50）を返し `truncated=true` とする。
    - `truncated=true` の場合、ユーザーは `page + 1` を指定して次のチャンクを取得できる。
  - パスはプロジェクトルート相対で `/` に統一し、`docs_search` は `docs/<relative>` を返す。  
  - クエリの AND 条件や正規表現、全文検索インデックス化は現時点では実装していない。将来的に追加する場合は DESIGN.md 上で明示的に仕様を拡張する。

- **取得 (`rules_page`, `rules_get`, `read_resource`)**  
  - `resolve_safe_path` → `resolve_rules_path` / `resolve_docs_path` で `has_absolute_or_rooted_component` / `..` / canonicalize チェックを順番に実施し、失敗は `PATH_ESCAPE`/`INVALID_INPUT` で返す。  
  - 取得対象がディレクトリでないことを確認し、`.md` であること、`MAX_FILE_BYTES` を超えないことを検証。超過時は `FILE_TOO_LARGE`。  
  - `read_resource` 系は `CancellationToken` を使い、IO 前後と `tokio::select!` で `token.cancelled()` を競合させてキャンセルを伝播させる。  
  - **ページネーション (`rules_get`, `docs_get`)**:
    - `page` 引数（デフォルト 1）を受け取る。
    - `MAX_LINES` (100) を1ページの固定サイズとし、`(page - 1) * MAX_LINES` 行をスキップして次の 100 行を取得する。
    - ファイル末尾に達していない（次ページがある）場合、取得できた行を返し `truncated=true` とする。
  - 取得の際 `path` を再入力した場合は `read_resource`（`typstlab://rules/<path>` または `typstlab://docs/<path>`）で同じファイルに必ず到達できる。

#### 5.10.6 エラーコード標準化

標準コードを以下に固定し、ツール横断で使用する。旧コードが必要な場合は互換マッピングを実装する。

| Code | 意味 | 旧コード例 |
| --- | --- | --- |
| INVALID_INPUT | 入力値不正（パス形式、空クエリ等） | - |
| NOT_FOUND | 対象リソース不存在 | - |
| PATH_ESCAPE | ルート外へ解決されるパス | PROJECT_PATH_ESCAPE |
| FILE_TOO_LARGE | サイズ上限超過 | FILE_TOO_LARGE |
| BUILD_FAILED | Typst ビルド失敗 | BUILD_FAILED |
| PROJECT_NOT_FOUND | typstlab.toml 不在 | PROJECT_NOT_FOUND |
| TYPST_NOT_RESOLVED | Typst バージョン解決失敗 | TYPST_NOT_RESOLVED |

**運用ルール（必須）**:

- **入力起因の失敗は必ず `INVALID_INPUT`**（例: 空クエリ、不正な paper_id、拡張子不正、必須フィールド欠落）。  
- **存在しない対象は `NOT_FOUND`**（例: ファイル/ディレクトリ不存在）。  
- **パス安全性違反は `PATH_ESCAPE`**（absolute/rooted/`..`/ルート外解決）。  
- **サイズ上限超過は `FILE_TOO_LARGE`**。  
- 返す `ErrorData` には **必ず `data.code` に標準コードを含める**（JSON-RPC の `code` とは別）。  
- `invalid_params` 相当のケースでも、クライアント互換性のため `data.code=INVALID_INPUT` を付与する。

#### 5.10.7 セキュリティチェック共通ルール

- パス検査は `has_absolute_or_rooted_component` による absolute/rooted 検知と `..` 禁止を必須とする。  
- canonicalize 後にプロジェクトルート配下であることを確認し、外部を指す場合は `PATH_ESCAPE`。  
- シンボリックリンクがルート外を指す場合はスキップまたは拒否。  
- エラーメッセージは「Path cannot be absolute or rooted」「Path cannot contain ..」「Path resolves outside root: <path>」の定型句を用いる。

#### 5.10.8 リソースとツールの責務分担

- コンテンツ取得の第一手段は `read_resource`（`typstlab://rules/*`, `typstlab://docs/*`）。  
- `rules_page` / `rules_get` は軽量閲覧用の補助ツールとして公開するが、同一パス検査・サイズ制限を適用する。  
- リソース上限: 1 MiB (`MAX_RESOURCE_BYTES`=1,048,576) を超える場合は `FILE_TOO_LARGE`。

**Rules リソース URI の正規スコープ**:

- `typstlab://rules/<path>` の `<path>` は **project root 相対パス**であり、`rules_*` 系ツールが返却する `path` を **そのまま埋め込める**ことを要件とする。  
- 許容パスは **以下のどちらか**で始まる必要がある。  
  1) `rules/`  
  2) `papers/<paper_id>/rules/`  
- `rules_*` 系ツールと **同一スコープ**であること（browse/search/list と read_resource の対象範囲は一致させる）。  
- **解釈規則**: `<path>` は `/` 区切りで表現する。  
  OS 依存の区切り文字を含む入力は **受理してもよい**が、内部で正規化して扱う。
- **例（許可）**:  
  - `typstlab://rules/rules/guidelines.md` → `rules/guidelines.md`  
  - `typstlab://rules/rules/subdir/guide.md` → `rules/subdir/guide.md`  
  - `typstlab://rules/papers/paper1/rules/citations.md` → `papers/paper1/rules/citations.md`  
- **注意**: `<path>` は **そのまま解決**される（paper 用の特別な推測は行わない）。  
- **例（禁止）**:  
  - `typstlab://rules/../secrets.md`（`..`）  
  - `typstlab://rules//absolute`（absolute/rooted）  
  - `typstlab://rules/papers/paper1/notes.md`（`papers/<id>/rules/` を経由していない）

#### 5.10.9 制限値と挙動
##### 5.10.9 Limits and Behavior

**Search Limits**:

- MAX_MATCHES: 50 (per page)
- MAX_SCAN_FILES: 50
- MAX_FILE_BYTES: 1MB

**Get Limits**:

- MAX_LINES: 100 (per page)

**Truncation Behavior**:

- **Search**:
  - Provide fixed-size pages (e.g. 50 items).
  - If results exceed the limit, return the items for the current page and set `truncated: true`.
  - User can request `page + 1` to retrieve the next chunk.
- **Get**:
  - Provide fixed-size pages (e.g. 100 lines).
  - Return content for current page range `(page-1)*limit` to `page*limit`.
  - If file ends before limit, `truncated: false`.
  - If more content exists, `truncated: true`.
- Do NOT clear results when truncated.

#### 5.10.10 テストマトリクス（TDD 参照用）

- 正常系: 存在する rules/docs で browse/search が成功し、`missing=false`。  
- missing 系: ルート未作成時に `missing=true` を返し、スキーマを固定。  
- 制限超過: `MAX_SCAN_FILES` 超過で `truncated=true` かつ結果は空配列、`MAX_MATCHES` 超過で `truncated=true` かつ結果は上限トリミング。  
- パス拒否: absolute/rooted/`..` で `INVALID_INPUT` or `PATH_ESCAPE`。  
- オフライン差分: offline では `list_tools` に `cmd_generate`/`cmd_build` を含まないこと。  
- symlink: ルート外を指す symlink は無視または拒否されること。
- **E2E パス互換**: `rules_*`/`docs_*` が返した `path` を再入力に使い、同一対象の閲覧が成功すること（OS 依存差を含める）。  
- **MCP 互換**: 返却 `path` を `read_resource` または対応ツールに渡す E2E を追加し、**最低 2 OS（macOS + Windows）**で確認する。  
- **E2E 対象例**:  
  - `rules_browse` の `path` → `read_resource`  
  - `rules_search` の `path` → `rules_page`/`read_resource`  
  - `docs_search` の `path` → `read_resource`  
  - `docs_browse` の `path` → `docs_search`
  - `rules_list` の `path` → `read_resource`  
  - `rules_page` の `path` → `read_resource`  
- **URI 例**: 返却 `path` はそのまま `typstlab://rules/<path>` / `typstlab://docs/<path>` に埋め込む。  
  - `rules_browse` が `path="rules/guidelines.md"` を返した場合 → `typstlab://rules/rules/guidelines.md`  
  - `rules_list` が `path="papers/paper1/rules/citations.md"` を返した場合 → `typstlab://rules/papers/paper1/rules/citations.md`  
  - `docs_search` が `path="docs/reference/syntax.md"` を返した場合 → `typstlab://docs/docs/reference/syntax.md`  
- **期待結果の明文化**:  
  - 返却 `path` の再入力は **常に同一ファイル**に解決されること。  
  - 同一ファイルに対して `rules_page` と `read_resource` の **内容整合**が取れること。  
  - 区切り文字差分（`/` vs `\\`）があっても再入力で **成功**すること。  
  - `missing=true` のケースは **再入力対象に含めない**（missing は再入力前提でない）。

---

### 5.11 Test Commands

#### 5.11.1 `typstlab test run [options]`

Typst テンプレートをコンパイルして検証する。

**Usage**:

```bash
typstlab test
typstlab test run
typstlab test run --paper report
typstlab test run --only-paper report
typstlab test run --only-project
typstlab test run --name report
```

**Options (v0.1)**:

- `--paper <paper-id>`: project + 指定 paper を対象にする
- `--only-paper <paper-id>`: 指定 paper のみ
- `--only-project`: project のみ
- `--name <name>`: test case 名でフィルタ（スコープ指定と併用可能）

**動作 (v0.1)**:

1. project と paper の test 定義を読み取る
   - デフォルトは **project + paper のマージ**
   - 同名 case は **paper が上書き**
2. `file-patterns` を展開（glob / 直パス）
3. 展開結果をソートして再現性を固定
4. 同一 case 内の重複を検出
   - `allow-duplicate = false` → warning
   - `allow-duplicate = true` → warning なし
5. `typst compile --root <project-root> <input> <output>` を実行
6. 出力先は `.typstlab/test-out/<case-name>/<file>.pdf`

**file-patterns ルール**:

- `!` が先頭の場合は除外（negation）
- `\!` はエスケープ（`!` を含むパスとして扱う）
- `"!"` 単体はエラー

**type (v0.1)**:

- `type = "compile"` のみ許可
- 未知の type はエラー

**Safety classification (v0.1)**:

- `network`: false
- `reads`: true
- `writes`: true（.typstlab/test-out の生成）
- `writes_sot`: false

**Exit code**: 成功 0, 失敗 1

#### 5.11.2 `typstlab test list [options]`

test case の一覧を表示する。

**Usage**:

```bash
typstlab test list
typstlab test list --paper report
typstlab test list --only-project
typstlab test list --name report
typstlab test list --detail
```

**Options (v0.1)**:

- `--paper <paper-id>`: project + 指定 paper を対象にする
- `--only-paper <paper-id>`: 指定 paper のみ
- `--only-project`: project のみ
- `--name <name>`: test case 名でフィルタ
- `--detail`: `file-patterns` と出力先を表示

**Safety classification (v0.1)**:

- `network`: false
- `reads`: true
- `writes`: false
- `writes_sot`: false

**Exit code**: 成功 0, 失敗 1

#### 5.11.3 Future Extensions (v0.2+)

- `type = "cmd"`: 任意コマンドの実行
- `type = "validate"`: 高速な PDF 検証（破損検出 + テキスト確認）
  - 画像 PDF は対象外（必要なら cmd で実行）

---

## 6. System Design

### 6.1 Typst Resolution Flow

#### 6.1.1 Resolution Order

```
1. state.json にキャッシュがあるか？
   YES → resolved_path が存在するか？
         YES → 使う（fast path）
         NO  → 再解決へ
   NO  → 再解決へ

2. 再解決フロー
   a. managed を探す
      場所：`{managed_cache_dir}/{version}/typst`（Windows は `typst.exe`）
      （`managed_cache_dir` は 6.1.2 を参照）
      条件：バージョン完全一致

   b. system を探す
      場所：$PATH から which typst
      条件：バージョン完全一致（typst --version で確認）

   c. どちらも見つからない
      → actions に "typstlab typst install {version}" を提示
```

#### 6.1.2 Managed Cache Directory

| OS | Path |
|----|------|
| macOS | `~/Library/Caches/typstlab/typst` |
| Linux | `~/.cache/typstlab/typst` |
| Windows | `%LOCALAPPDATA%\typstlab\typst` |

構造（macOS の例）：

```
~/Library/Caches/typstlab/typst/
  0.13.1/
    typst              # バイナリ
  0.12.0/
    typst
```

**パス表記のポリシー**：

- state.json / status --json の `resolved_path` は **OS ネイティブの区切り文字**を使う
  - macOS/Linux: `/`（例: `/Users/alice/Library/Caches/typstlab/typst/0.13.1/typst`）
  - Windows: `\`（例: `C:\Users\alice\AppData\Local\typstlab\typst\0.13.1\typst.exe`）
- 仕様書内で `{managed_cache_dir}` 等の記号を使う箇所は `/` で統一（説明用）

**環境変数ポリシー**：

- managed cache のパスは **環境変数（XDG_CACHE_HOME 等）を上書きして解決してはならない**
- typstlab は OS の標準的 cache location を **常に優先**する
- これにより、エージェントが「どこに typst があるか」を確実に推論できる

#### 6.1.3 System Typst Validation

- 初回のみ検証（`typstlab typst link`）
- state.json にキャッシュ
- 以降は resolved_path の存在チェックのみ（fast）
- 明示的な再検証は `typstlab typst link -f`

### 6.2 Project Detection

#### 6.2.1 Search Algorithm

Git 方式：上位ディレクトリを辿って `typstlab.toml` を探す。

```rust
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        let config = current.join("typstlab.toml");
        if config.exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}
```

#### 6.2.2 Monorepo Handling

v0.1 では：

- 最初に見つかった typstlab.toml を使う
- ネストは非推奨だが、技術的には可能

将来：

- monorepo 対応を検討
- でもファイルサイズ肥大化のリスクあり

### 6.3 Error Handling

#### 6.3.1 Exit Code Policy

| コマンド | 成功 | 失敗 |
|---------|------|------|
| `status` | exit 0 | exit 0（JSON 内でエラー） |
| `doctor` | exit 0 | exit 0（JSON 内でエラー） |
| `build` | exit 0 | exit 1 |
| `watch` | exit 0 | exit 1（起動失敗時） |
| `new` | exit 0 | exit 1 |
| `generate` | exit 0 | exit 1 |
| `typst install` | exit 0 | exit 1 |
| その他実行系 | exit 0 | exit 1 |

#### 6.3.2 Error Response Format

status/doctor の失敗時（exit 0）:

```json
{
  "schema_version": "1.0",
  "project": null,
  "timestamp": "2026-01-05T12:34:56Z",
  "checks": [
    {
      "id": "project_not_found",
      "name": "Project detection",
      "status": "error",
      "message": "typstlab.toml not found in current or parent directories"
    }
  ],
  "actions": [
    {
      "id": "init_project",
      "command": "typstlab new <project-name>",
      "description": "Initialize a new typstlab project",
      "enabled": true,
      "safety": {
        "network": false,
        "writes": true,
        "reads": false
      },
      "prerequisite": null
    }
  ]
}
```

実行系コマンドの失敗時（exit 1）:

```json
{
  "error": {
    "code": "BUILD_FAILED",
    "message": "Typst compilation failed",
    "details": {
      "paper": "report",
      "typst_error": "..."
    }
  }
}
```

#### 6.3.3 Error Code Namespace

| Prefix | 意味 |
|--------|------|
| `PROJECT_*` | プロジェクト構造・検出 |
| `TYPST_*` | Typst 関連 |
| `BUILD_*` | ビルドエラー |
| `NETWORK_*` | ネットワークポリシー違反 |
| `PAPER_*` | Paper 関連 |
| `CONFIG_*` | 設定ファイル関連 |

### 6.4 state.json Management

#### 6.4.1 Role

state.json は「このマシンで typstlab を正しく・高速に動かすための、破棄可能な実行状態キャッシュ」。

**特徴**:

- 破棄可能（削除しても再生成される）
- マシン固有
- gitignore 対象
- checked_at は記録のみ（基本的に信頼）

#### 6.4.2 Update Timing

| フィールド | 更新タイミング |
|-----------|--------------|
| `typst.*` | typst link, typst install 実行時 |
| `uv.*` | link uv 実行時 |
| `docs.*` | typst docs sync 実行時 |
| `build.last` | build 完了時 |

#### 6.4.3 Validation

- resolved_path の存在チェックは毎回（cheap）
- バージョン検証は省略（コスト高い）
- debug ビルドでは厳密に検証（optional）

#### 6.4.4 Schema Evolution

```rust
match state.schema_version {
    "1.0" => { /* current */ },
    "0.9" => { /* migrate */ },
    _ => {
        warn!("Unknown schema version, regenerating");
        State::empty()
    }
}
```

- 破棄可能なので migration は緩く
- 失敗したら空の State を返す

### 6.5 bin/ Shim Implementation

#### 6.5.1 bin/typst

```bash
#!/bin/sh
# AUTO-GENERATED by typstlab
# DO NOT EDIT

BIN_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT_DIR="$(CDPATH= cd -- "$BIN_DIR/.." && pwd)"

if [ ! -f "$ROOT_DIR/typstlab.toml" ]; then
  echo "Error: typstlab.toml not found next to bin/ (expected project root at $ROOT_DIR)" >&2
  exit 1
fi

cd "$ROOT_DIR" || exit 1
exec typstlab typst exec -- "$@"
```

**役割**:

- プロジェクトルート検証
- typstlab 経由で実行（契約の強制）
- （将来）監査ログ、ネットワークポリシー

**注意**:

- ネットワークポリシーは v0.1 では適用しない（Typst バージョン追従が大変）

#### 6.5.2 bin/uv

```bash
#!/bin/sh
# AUTO-GENERATED by typstlab
# DO NOT EDIT

# プロジェクトルートの検証
if [ ! -f "typstlab.toml" ]; then
  echo "Error: Must run from typstlab project root" >&2
  exit 1
fi

# typstlab 経由で実行
exec typstlab uv exec -- "$@"
```

**役割**:

- プロジェクトルート検証
- typstlab 経由で実行（契約の強制）
- jq 依存を排除（state.json 読み取りは typstlab 内部で処理）

### 6.6 Watch Implementation

#### 6.6.1 Dependency Tracking

v0.1 では簡易的なパターンマッチ：

**監視対象**（`--paper report` の場合）:

- `papers/report/main.typ`
- `papers/report/paper.toml`
- `papers/report/sections/*.typ`
- `papers/report/assets/*`
- `refs/sets/*/library.bib`
- `refs/sets/*/sources.lock`
- `figures/*`
- `layouts/<layout-name>/*`

#### 6.6.2 Change Detection Flow

```
1. notify crate でファイルシステム監視
   ↓
2. 変更検知
   ↓
3. debounce (500ms)
   ↓
4. paper.toml が変更された？
   YES → typstlab generate --paper <id>
   NO  → スキップ
   ↓
5. typstlab build --paper <id>
   ↓
6. Typst の incremental compilation に任せる
```

#### 6.6.3 Hash Management

- Typst の incremental compilation に任せる
- typstlab は hash 管理しない（シンプルに）

### 6.7 Safety Scope Definitions

#### 6.7.1 Network Scope

network policy が影響するのは **typstlab 自身が行う通信のみ**：

- ✅ Typst バイナリのダウンロード
- ✅ Typst docs (MD) 取得
- ✅ refs fetch（DOI/URL からの取得）
- ✅ refs touch（source への再アクセス）
- ✅ MCP web fetch（将来）

**対象外**（制御不可能）：

- ❌ Typst 本体の @preview/ パッケージダウンロード
- ❌ Python scripts のネットワーク通信
- ❌ uv が内部で行うネットワーク通信（v0.1 では typstlab は制御しない）
- ❌ OS レベル通信

**詳細は 3.4.4 の network のスコープを参照**。

#### 6.7.2 Reads Scope

`reads: true` が示すのは **プロジェクトルート配下のプロジェクトデータ読み取りのみ**：

- ✅ typstlab.toml, paper.toml, main.typ 等のプロジェクトファイル
- ✅ refs/, layouts/, papers/ 配下の全ファイル
- ✅ .typstlab/state.json, bin/ 等の派生物（プロジェクトデータ）
- ❌ managed cache（`~/Library/Caches/typstlab/...` 等）— ツールチェーン解決の内部実装
- ❌ system binary（`/usr/bin/typst` 等）— ツールチェーン解決の内部実装

**重要な区別**：

- `reads: false` は「プロジェクトデータを読まない」であり、「ファイルシステムを一切読まない」ではない
- ツールチェーン解決（typst/uv の検証等）のためのローカル参照は `reads` の分類対象外

**symlink ポリシー**：

- typstlab の自前走査は `follow_links = false`
- 直接指定されたファイルが symlink の場合は読み取り許可
- 実体がプロジェクトルート外なら `PROJECT_PATH_ESCAPE` エラー

**詳細は 3.4.4 の reads のスコープを参照**。

#### 6.7.3 Policy Values

| Value | 意味 |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `"auto"` | 通信を許可（デフォルト） |
| `"never"` | typstlab 自身のネットワーク通信を禁止。ネットワークが必要なコマンドは実行時にエラー。status/doctor は actions を列挙し enabled=false と disabled_reason で示す |

#### 6.7.4 Network Policy Enforcement

- `network = "never"` 時：
  - `typstlab typst install` → エラー
  - `typstlab typst docs sync` → エラー
  - `typstlab refs fetch` → エラー
  - `typstlab refs touch` → エラー
  - actions に network=true なものは `enabled: false, disabled_reason: "network policy is 'never'"` で列挙
    - 例: `typst_install`, `docs_sync`, `refs_fetch`, `refs_touch`

### 6.8 Path Security (rules/)

**Prevention of Path Escaping**:

1. No `..` in paths
2. No absolute paths
3. Symlinks: direct file access allowed, but must resolve within project root
4. Error: `PROJECT_PATH_ESCAPE` if resolution fails

**Implementation**:

```rust
fn validate_path(project_root: &Path, requested: &Path) -> Result<PathBuf> {
    let canonical = requested.canonicalize()?;
    if !canonical.starts_with(project_root) {
        return Err(TypstlabError::ProjectPathEscape {
            path: requested.to_path_buf()
        });
    }
    Ok(canonical)
}
```

**Design Rationale**:

- `canonicalize()` resolves symlinks and normalizes paths
- `starts_with()` check ensures path is within project root
- This prevents path traversal attacks while allowing legitimate symlinks
- Error message includes the attempted path for debugging

**rules/ specific constraints**:

- Only `.md` files are accessible
- Hidden files (starting with `.`) are excluded
- Directory traversal with `walkdir` uses `follow_links = false`
- Direct file access via symlinks is validated with canonicalization

### 6.9 Process Synchronization (File Locking)

#### 6.9.1 Overview

typstlab uses **advisory file locks** (via `fs2` crate) to prevent race conditions when multiple processes access shared resources concurrently. This ensures safe parallel execution of commands like `typst install`, `docs sync`, and `build`.

**Key Design Principles**:

- **RAII pattern**: Locks are automatically released when guard goes out of scope
- **Advisory locks**: Processes cooperate voluntarily (not enforced by OS)
- **Cross-platform**: Works on Unix (flock) and Windows (LockFileEx)
- **Timeouts**: All lock acquisitions have timeouts (30-120 seconds)
- **Early exit optimization**: Operations check for completion before acquiring locks

#### 6.9.2 Lock Targets and Placement

| Resource               | Lock File                                | Scope       | Timeout |
| ---------------------- | ---------------------------------------- | ----------- | ------- |
| **state.json updates** | `.typstlab/state.lock`                   | Per-project | 30s     |
| **Docs sync**          | `.typstlab/kb/docs.lock`                 | Per-project | 120s    |
| **Typst install**      | `{managed_cache}/{version}/install.lock` | Per-version | 60s     |

**Lock File Naming Convention**:

- Descriptive names (not just `.lock`)
- Purpose-specific: `state.lock`, `docs.lock`, `install.lock`
- Hidden (start with `.`) to avoid clutter
- Added to `.gitignore` (auto-cleaned on process exit)

#### 6.9.3 Implementation Pattern

```rust
use std::time::Duration;
use typstlab_core::lock::acquire_lock;

// Example: Protecting state.json updates
pub fn save_state(&self, path: &Path) -> Result<()> {
    let lock_path = path.parent()
        .ok_or("Invalid state.json path")?
        .join("state.lock");

    // Acquire lock with timeout (RAII guard)
    let _guard = acquire_lock(
        &lock_path,
        Duration::from_secs(30),
        "Updating state.json"
    )?;

    // Critical section: atomic write
    let temp_file = NamedTempFile::new_in(path.parent().unwrap())?;
    temp_file.write_all(self.to_json_bytes())?;
    temp_file.persist(path)?;  // Atomic rename

    Ok(())  // Lock auto-released via Drop
}
```

#### 6.9.4 Lock Acquisition Strategy

**Retry with Exponential Backoff**:

- Initial retry delay: 10ms
- Max retry delay: 500ms
- Progress message after 2 seconds: "Waiting for lock on ..."

**Timeout Behavior**:

- If lock cannot be acquired within timeout → `LockError::Timeout`
- Error message includes lock description for debugging

**Early Exit Optimization**:

- **Docs sync**: Check if docs directory exists with files → early exit (no lock needed)
- **Typst install**: Check if binary exists → early exit (idempotency)
- This reduces lock contention for common cases

#### 6.9.5 Lock Scope Design Rationale

**Per-Project Locks** (state.json, docs):

- Different projects can run simultaneously without conflict
- Example: `project-a/` and `project-b/` can both run `docs sync` in parallel

**Per-Version Locks** (typst install):

- Different versions can install simultaneously: `typst install 0.12.0` and `typst install 0.13.0` in parallel
- Same version serializes: second process waits for first to complete, then exits early

#### 6.9.6 Limitations and Considerations

**Known Limitations**:

- **Network filesystems (NFS, SMB)**: Advisory locks may be slower or unreliable
  - Workaround: Use local cache directories when possible
- **Stale locks**: If process crashes, OS automatically releases locks
  - No manual cleanup needed (advisory locks are process-bound)

**Not Covered**:

- **Thread-level locking**: Rust's type system prevents most intra-process races
- **Git operations**: Handled by Git's own locking mechanisms
- **Typst compilation**: Read-only operations, no locking needed

**Future Enhancements** (v0.2+):

- **Lock metadata**: Store PID, hostname, purpose in lock file content
- **NFS fallback**: Detect network FS and use alternative strategies
- **Distributed locks**: For multi-machine environments (research phase)

#### 6.9.7 Testing Requirements

**Unit Tests** (crates/typstlab-core/src/lock/tests.rs):

- Lock acquisition success
- Concurrent access blocking (thread-level)
- Timeout behavior
- RAII cleanup on drop

**Integration Tests** (crates/typstlab/tests/*):

- Parallel installs (same version) → second waits, both succeed
- Parallel docs sync (same project) → one downloads, others early exit
- Concurrent builds (different papers) → parallel execution

**CI Requirements**:

- All tests must pass on macOS, Linux, Windows
- Tests run in parallel by default (no --test-threads=1)

---

## 7. Implementation Guide

### 7.1 Cargo Workspace Structure

```
typstlab/
  Cargo.toml                  # workspace root

  crates/
    typstlab/                 # メイン CLI
      src/
        main.rs
        commands/
          new.rs
          build.rs
          watch.rs
          status.rs
          doctor.rs
          ...
        output.rs             # 出力フォーマット

    typstlab-core/            # コアロジック（唯一の真実）
      src/
        lib.rs
        project/              # プロジェクト管理
          mod.rs
          layout.rs
        paper/                # paper 管理
          mod.rs
          model.rs
        config/               # 設定管理
          mod.rs
          model.rs
        status/               # status/doctor
          mod.rs
          schema.rs
          engine.rs
          checks/
            mod.rs
            env.rs
            typst.rs
            build.rs
            refs.rs
        error.rs

    typstlab-typst/           # Typst 統合
      src/
        lib.rs
        resolve.rs            # 解決ロジック
        exec.rs               # 実行
        info.rs               # バージョン情報
        install/
          mod.rs
          release.rs          # GitHub Release
          cargo.rs            # cargo fallback

    typstlab-watch/           # watch 最適化
      src/
        lib.rs
        watcher.rs
        debounce.rs

    typstlab-mcp/             # MCP サーバ
      src/
        lib.rs
        server.rs
        tools/
          mod.rs
          status.rs
          build.rs
          watch.rs
          refs.rs

    typstlab-shim/            # bin/ 生成
      src/
        lib.rs
        generate.rs

    typstlab-testkit/         # テストユーティリティ
      src/
        lib.rs
```

### 7.2 Implementation Priority

#### Phase 1: 基礎（Week 1-2）

1. **typstlab-core の基本型定義**
   - `Project`, `Paper`, `Config` 構造体
   - TOML パース（serde）
   - `State` 管理
   - エラー型定義

2. **プロジェクト検出ロジック**
   - `find_project_root()`
   - 上位ディレクトリ探索

3. **state.json 読み書き**
   - `State::load()`, `State::save()`
   - schema evolution

#### Phase 2: Typst 統合（Week 2-3）

1. **typstlab-typst の解決フロー**
   - `resolve_typst()` 実装
   - managed / system 探索
   - バージョン検証

2. **typst install 実装**
   - GitHub Release からダウンロード
   - tar.gz 展開
   - cargo fallback

3. **bin/typst shim 生成**
   - typstlab-shim 実装
   - `typst exec` 実装

#### Phase 3: コマンド実装（Week 3-4）

1. **status コマンド**
   - checks 実装
   - actions 生成
   - JSON 出力

2. **doctor コマンド**
   - ツールチェーンチェック
   - プロジェクト構造検証

3. **new コマンド**
   - プロジェクト雛形生成
   - paper 雛形生成
   - layouts コピー

4. **generate コマンド**
    - layouts 解決
    - _generated/ 生成
    - テンプレート置換

#### Phase 4: CLI統合とポリッシュ（Week 4-6）

1. **build コマンド**
    - generate 統合
    - typst compile 実行
    - state.json 更新

2. **sync コマンド**
    - link typst 統合
    - generate 統合
    - doctor actions 実行

3. **CLI統合**
    - doctor コマンド統合
    - status コマンド統合
    - エラーハンドリング

4. **MCP サーバ完成**
    - rules tools（read_rules, search_rules）
    - status tool（v0.1では基本機能）
    - --offline モード

5. **E2Eテスト**
    - 統合テスト完成
    - ドキュメント整備

#### v0.2で実装予定

1. **watch コマンド**
    - typstlab-watch 実装
    - notify crate 統合
    - debounce とファイル監視

2. **uv 統合**
    - link uv 実装
    - uv exec 実装
    - pyproject.toml / uv.lock 管理

3. **refs コマンド**
    - DOI 取得（CrossRef API）
    - URL 取得（スクレイピング）
    - library.bib 更新
    - sources.lock 管理

4. **MCP 追加ツール**
    - build tool
    - watch tool

### 7.3 Key Dependencies

```toml
[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# File system
notify = "6"  # watch
walkdir = "2"  # project search

# HTTP
reqwest = { version = "0.11", features = ["blocking"] }

# Error handling
anyhow = "1"
thiserror = "1"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# Hashing (if needed)
sha2 = "0.10"

# MCP
# (MCP crate 検討)
```

### 7.4 Testing Strategy

#### Unit Tests

- 各 crate の `src/` に `#[cfg(test)] mod tests`
- 特に typstlab-core, typstlab-typst

#### Integration Tests

- `tests/` ディレクトリ
- 実際のプロジェクト雛形を使ったテスト
- typstlab-testkit でヘルパー提供

#### E2E Tests

- 実際の CLI コマンド実行
- `assert_cmd` crate 使用

### 7.5 Documentation

- `README.md`: ユーザー向け
- `DESIGN.md`: このドキュメント（実装参照）
- `CONTRIBUTING.md`: コントリビューター向け
- 各 crate の `lib.rs` に crate-level doc
- 公開 API に rustdoc

---

## Appendix

### A. Glossary

| 用語 | 意味 |
|------|------|
| **正（Source of Truth）** | 情報の唯一の信頼できる源 |
| **派生物** | 正から生成されるもの（破棄・再生成可能） |
| **要求（requirements）** | typstlab.toml に記述される、求められる仕様 |
| **解決（resolved）** | state.json に記録される、実際に使用する実体 |
| **managed** | typstlab が管理するキャッシュディレクトリ |
| **system** | OS にインストールされているツール |
| **shim** | 実体を隠蔽して契約を強制する薄いラッパー |
| **layout** | _generated/ を生成するためのテンプレート集合 |

### B. References

- Typst: <https://typst.app/>
- uv: <https://github.com/astral-sh/uv>
- MCP (Model Context Protocol): <https://modelcontextprotocol.io/>
- XDG Base Directory: <https://specifications.freedesktop.org/basedir-spec/>

---

**End of Document**
