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

### 1.3 Scope (v0.1)

**含むもの**：

- PDF ビルドのみ
- watch 最適化
- プロジェクト / paper / status の骨格
- Typst バージョン固定（プロジェクト単位）
- Typst docs（MD）取得（MCP提供用）
- MCP サーバ（最小）
- uv は「リンク（解決）と診断」まで（自動インストール/自動実行はしない）
- refs 管理（fetch/check/touch）と sources.lock

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
  pyproject.toml         # Python環境宣言（正）
  uv.lock                # Python依存ロック（正）

  layouts/               # プロジェクトレベルのレイアウト定義
    default/
      meta.typ
      header.typ
      refs.typ
    ieee/
      meta.typ
      header.typ
      refs.typ

  refs/
    library.bib
    sources.lock         # 取得日ベース（+可能ならhash）

  scripts/               # 図表・表の生成スクリプト置き場（実行はv0.2以降）
  data/                  # 入力データ（原則immutable）
  figures/               # 生成物（図）
  tables/                # 生成物（表）

  dist/                  # 出力集約先（正）
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
| 参考文献 | **正** | `refs/library.bib` | git commit |
| 参考文献履歴 | **正** | `refs/sources.lock` | git commit |
| レイアウト | **正** | `layouts/**/*.typ` | git commit |
| 出力物 | **派生物** | `dist/**/*.pdf` | gitignore、再生成可能 |
| 生成コード | **派生物** | `papers/*/_generated/*.typ` | gitignore、再生成可能 |
| bin shim | **派生物** | `bin/typst`, `bin/uv` | gitignore、再生成可能 |
| Typst docs | **観測物** | `.typstlab/kb/typst/docs/` | gitignore、再取得可能 |
| 実行状態 | **観測キャッシュ** | `.typstlab/state.json` | gitignore、破棄可能 |

**再現性の原則**：
再現性の正は **設定・規約・入力ファイルのみ** で構成される。
出力物（PDF、`_generated/` 等）は常に **再生成可能な派生物** である。
観測物・観測キャッシュは、環境の状態を記録するが、再現性を決定しない。

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

# 外部ツール
[tools]
uv = { required = true }  # バージョン指定なし = any
# 将来的に拡張可能

# v0.1 で uv を required とする理由：
# - pyproject.toml / uv.lock は「正（SOT）」として git commit される
# - つまり Python 環境は「再現性の正」の一部
# - scripts 実行は v0.2 以降だが、環境の宣言自体は v0.1 から必須
# - doctor/sync は uv が存在することを前提に動作する

# ネットワークポリシー
[network]
policy = "auto"  # "auto" | "never"

# ビルドのデフォルト設定
[build]
parallel = true  # 複数 paper の並行ビルド（将来）

# watch のデフォルト設定
[watch]
debounce_ms = 500
ignore = ["*.tmp", ".DS_Store", "*.swp"]
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
name = "ieee"  # layouts/ieee/ を使う（省略時は "default"）

# 出力設定
[output]
name = "report"  # 拡張子なし → dist/report/report.pdf

# ビルド設定
[build]
targets = ["pdf"]  # v0.1 では pdf のみ

# 参考文献（optional）
[refs]
bibliography = ["library.bib"]  # refs/ からの相対パス
# 空配列可能だが、空ならセクション自体を省略推奨
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
      "source": "official"
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
        "reads": true
      },
      "prerequisite": null
    },
    {
      "id": "build_paper",
      "command": "typstlab build --paper report",
      "description": "Build this paper to PDF",
      "enabled": false,
      "disabled_reason": "prerequisite action 'install_typst' not completed",
      "safety": {
        "network": false,
        "writes": true,
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
| `safety` | object | ✅ | 副作用の宣言 |
| `prerequisite` | string[] \| null | ✅ | 依存する action ID |

**enabled と disabled_reason**：

- **enabled: true**: 現在実行可能
- **enabled: false**: 実行不可能、disabled_reason で理由を説明
  - 例: `"network policy is 'never'"`
  - 例: `"prerequisite action 'install_typst' not completed"`

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
    meta.typ                  # paper メタ情報の型定義
    header.typ                # ドキュメント設定・ヘッダー
    refs.typ                  # 参考文献設定
  ieee/                       # IEEE スタイル
    meta.typ
    header.typ
    refs.typ
  minimal/                    # ミニマル
    meta.typ
    refs.typ
    # header.typ なし（不要なものは省略可能）
```

### 4.3 Resolution Order (v0.1)

v0.1 では以下の順序で解決：

1. `layouts/<layout-name>/<file>` (project-level)
2. builtin layouts

**将来（v0.2+）**：

1. `papers/<id>/layouts/<file>` (paper-level、最優先)
2. `layouts/<layout-name>/<file>` (project-level)
3. builtin layouts

### 4.4 Layout Files

#### 4.4.1 meta.typ

paper.toml から動的生成される。paper のメタ情報を Typst の dict として定義。

**テンプレート例** (`layouts/default/meta.typ`):

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

#### 4.4.3 refs.typ

参考文献設定。bibliography パスを置換。

**テンプレート例** (`layouts/default/refs.typ`):

```typst
// AUTO-GENERATED by typstlab from paper.toml

{{ BIBLIOGRAPHY }}
```

**生成例** (`papers/report/_generated/refs.typ`):

```typst
// AUTO-GENERATED by typstlab from paper.toml

#bibliography("../../refs/library.bib")
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
   - paper.toml の [layout] name を取得
   - layouts/<name>/ を探す
   - なければ builtin を使う
   ↓
3. _generated/meta.typ を生成
   - layouts/<name>/meta.typ をテンプレートとして使用
   - paper.toml の値で {{ PLACEHOLDERS }} を置換
   ↓
4. _generated/header.typ をコピー
   - layouts/<name>/header.typ をそのままコピー
   ↓
5. _generated/refs.typ を生成
   - layouts/<name>/refs.typ をテンプレートとして使用
   - paper.toml の [refs] で {{ BIBLIOGRAPHY }} を置換
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
- **唯一の生成者**：`typstlab generate` のみが生成・更新できる
- **破棄可能**：削除しても `generate` で再生成できる

**CI での扱い**:

```yaml
# CI の例
steps:
  - checkout
  - run: typstlab sync              # ツールチェーン準備
  - run: typstlab generate --all    # _generated/ を生成（必須）
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
| 実行系（build, watch, new, generate） | exit 0 | exit 1 | CI/CD, 人間の利用 |

### 5.2 Project Commands

#### 5.2.1 `typstlab new <project-name>`

新しい typstlab プロジェクトを作成する。

**Usage**:

```bash
typstlab new my-research
```

**動作**:

1. `<project-name>/` ディレクトリを作成
2. `typstlab.toml` を生成（デフォルト値）
3. 必須ディレクトリを作成（`layouts/`, `refs/`, `papers/`, `dist/`, etc.）
4. builtin layouts をコピー（`layouts/default/`, `layouts/minimal/`）
5. `.typstlab/` を初期化

**Exit code**: 成功 0, 失敗 1

#### 5.2.2 `typstlab new paper <paper-id>`

新しい paper を作成する。

**Usage**:

```bash
typstlab new paper report --title "My Report" --layout ieee
```

**Options**:

- `--title <title>`: paper のタイトル（必須または対話的入力）
- `--layout <name>`: レイアウト名（省略時は default）
- `--author <name>`: 著者名（省略時は project.default_author）

**動作**:

1. `papers/<paper-id>/` を作成
2. `paper.toml` を生成
3. `main.typ` を生成（初期テンプレート）
4. `_generated/` を生成（layouts から）
5. `sections/`, `assets/` を作成

**Exit code**: 成功 0, 失敗 1

### 5.3 Build Commands

#### 5.3.1 `typstlab build --paper <id>`

指定した paper をビルドする。

**Usage**:

```bash
typstlab build --paper report
typstlab build --paper report --full
```

**Options**:

- `--paper <id>`: ビルドする paper の ID（必須）
- `--full`: 強制的に _generated/ を再生成

**動作**:

1. paper.toml を読む
2. _generated/ が古ければ再生成（--full なら常に）
3. `typst compile papers/<id>/main.typ dist/<id>/<output_name>.pdf`
4. state.json の build.last を更新

**Exit code**: 成功 0, 失敗 1

#### 5.3.2 `typstlab watch --paper <id>`

指定した paper の変更を監視して自動ビルドする。

**Usage**:

```bash
typstlab watch --paper report
```

**動作**:

1. 依存ファイルを監視（main.typ, sections/, assets/, refs/, figures/, tables/）
2. paper.toml の変更も監視
3. 変更検知 → debounce (500ms) → build
4. Typst の incremental compilation に任せる

**Exit code**: 中断まで実行（Ctrl-C で exit 0）

### 5.4 Status Commands

#### 5.4.1 `typstlab status [--paper <id>] [--json]`

プロジェクトまたは paper の状態を取得する。

**Usage**:

```bash
typstlab status                    # プロジェクト全体
typstlab status --paper report     # 特定 paper
typstlab status --paper report --json
```

**動作**:

1. プロジェクトルートを検出
2. typstlab.toml, state.json を読む
3. checks を実行（Typst 解決、paper 存在、etc.）
4. actions を提案
5. JSON または人間向けフォーマットで出力

**Exit code**: 常に 0

#### 5.4.2 `typstlab doctor [--json]`

ツールチェーンと環境の健全性を診断する。

**Usage**:

```bash
typstlab doctor
typstlab doctor --json
```

**動作**:

1. Typst の可用性チェック
2. uv の可用性チェック
3. docs の整合性チェック
4. プロジェクト構造の検証
5. 修復方法を actions で提示

**Exit code**: 常に 0

### 5.5 Generate Command

#### 5.5.1 `typstlab generate [--paper <id>] [--all]`

_generated/ を生成・更新する（ビルドはしない）。

**Usage**:

```bash
typstlab generate --paper report   # 特定 paper
typstlab generate --all            # 全 paper
```

**動作**:

1. paper.toml を読む
2. layout を解決
3. _generated/ を生成
4. state.json は更新しない（ビルドしていないので）

**Exit code**: 成功 0, 失敗 1

### 5.6 Sync Command

#### 5.6.1 `typstlab sync [--apply]`

プロジェクトが想定する環境でビルド可能な状態に到達することを保証する。

**Usage**:

```bash
typstlab sync              # デフォルトモード（SOT 非変更）
typstlab sync --apply      # ネットワーク通信・managed install を許可
```

**sync が保証する到達状態（v0.1）**:

- Typst が解決済み（要求バージョンと一致）
- uv が解決済み（required の場合）
- bin/ shim が存在
- _generated/ が最新
- docs が存在し、バージョン整合（--apply 時のみ取得）

**動作（デフォルトモード）**:

1. `typstlab typst link` を実行（Typst 解決）
2. `typstlab link uv` を実行（uv 解決）
3. bin/ shim を生成
4. `typstlab generate --all` を実行（全 paper の _generated/ 更新）
5. state.json を更新

**動作（--apply モード）**:

上記に加えて：

- `typstlab doctor --json` を実行
- 以下の actions を自動実行（v0.1 で固定）:
  - `typstlab typst install <version>`（Typst が未解決の場合のみ）
  - `typstlab typst docs sync`（docs が不整合の場合のみ）

**重要な原則**:

- デフォルトモードは **SOT（正）を変更しない**
- `.typstlab/`, `_generated/`, `bin/` は派生物なので生成・上書きする
- `--apply` なしではネットワーク通信を行わない
- 既存の SOT（refs/, papers/, typstlab.toml 等）は変更しない

**冪等性（idempotency）**:

- `sync` は同じ project に対して複数回実行しても結果は変わらない
- 副作用は `.typstlab/`, `_generated/`, `bin/` に限定される
- エージェントは安心して何度でも `sync` を呼べる

**Exit code**: 成功 0, 失敗 1

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

#### 5.7.4 `typstlab typst exec -- <args>`

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
2. docs を取得（ソースは要検討）
3. `.typstlab/kb/typst/docs/` に保存
4. state.json を更新

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

**Exit code**: uv の exit code をそのまま返す（未解決時は exit 1）

### 5.9 Refs Commands

refs コマンドは、参考文献の取得と履歴管理を担当する。
論文用途を尊重し、**アクセス日時を first-class** に扱う。

#### 5.9.1 `typstlab refs fetch --doi <doi> | --url <url>`

DOI または URL から BibTeX を新規取得して refs/library.bib に追加する。

**Usage**:

```bash
typstlab refs fetch --doi 10.1234/example
typstlab refs fetch --url https://arxiv.org/abs/2301.00000
```

**Options**:

- `--doi <doi>`: DOI から取得
- `--url <url>`: URL から取得

**動作**:

1. DOI または URL から BibTeX を取得
   - DOI: CrossRef API など
   - URL: arXiv, 論文サイト等（可能な範囲）
2. refs/library.bib に追加
3. refs/sources.lock に取得履歴を記録
   - source（DOI または URL）
   - fetched_at（取得日時）
   - hash/etag（optional）

**重要な原則**:

- 取得日時が**情報源の信頼性の証明**となる
- 「このタイミングで絶対に正しい」ことを保証する

**Exit code**: 成功 0, 失敗 1

#### 5.9.2 `typstlab refs check`

既存の refs の整合性を検証する（ネットワーク不要）。

**Usage**:

```bash
typstlab refs check
```

**動作**:

1. refs/library.bib を読む
2. refs/sources.lock と突き合わせ
3. 不整合を検出
   - library.bib にあるが sources.lock にない
   - sources.lock にあるが library.bib にない
   - cite されているが library.bib にない（main.typ を parse）
4. 結果を報告

**Exit code**: 成功 0, 失敗 1

#### 5.9.3 `typstlab refs touch [--key <key>] [--all]`

既存の refs に今日の日付で再アクセスして記録を更新する。

**Usage**:

```bash
typstlab refs touch --key smith2020     # 特定エントリ
typstlab refs touch --all               # 全エントリ
```

**動作（v0.1）**:

1. sources.lock から対象エントリを取得
2. source（DOI/URL）に再アクセス
3. sources.lock の last_accessed を今日の日付に更新
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

**Network Policy**:

- `network = "never"` 時はエラー
- status/doctor の actions では `enabled: false, disabled_reason: "network policy is 'never'"` で表示

**Exit code**: 成功 0, 失敗 1

#### 5.9.4 sources.lock の役割

refs/sources.lock は参考文献取得・アクセスの履歴を記録する。

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

**提供するツール（v0.1）**:

- `status`: プロジェクト/paper の状態取得
- `build`: paper のビルド
- `watch`: paper の監視（将来）
- `typst_docs_status`: docs の状態（将来）

**--offline モード**:

- safety.network=true なツールは提供しない
- tools/list で filtered リストを返す

**Exit code**: 中断まで実行

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

# プロジェクトルートの検証
if [ ! -f "typstlab.toml" ]; then
  echo "Error: Must run from typstlab project root" >&2
  exit 1
fi

# typstlab 経由で実行
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
- `refs/library.bib`
- `figures/*`
- `tables/*`
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

### 6.7 Network Policy

#### 6.7.1 Scope

network policy が影響するのは **typstlab 自身が行う通信のみ**：

- ✅ Typst バイナリのダウンロード
- ✅ Typst docs (MD) 取得
- ✅ refs fetch（DOI/URL からの取得）
- ✅ refs touch（source への再アクセス）
- ✅ MCP web fetch（将来）

**対象外**（制御不可能）：

- ❌ Typst 本体の @preview/ パッケージダウンロード
- ❌ Python scripts のネットワーク通信
- ❌ OS レベル通信

#### 6.7.2 Policy Values

| Value | 意味 |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `"auto"` | 通信を許可（デフォルト） |
| `"never"` | typstlab 自身のネットワーク通信を禁止。ネットワークが必要なコマンドは実行時にエラー。status/doctor は actions を列挙し enabled=false と disabled_reason で示す |

#### 6.7.3 Enforcement

- `network = "never"` 時：
  - `typstlab typst install` → エラー
  - `typstlab typst docs sync` → エラー
  - `typstlab refs fetch` → エラー
  - `typstlab refs touch` → エラー
  - actions に network=true なものは `enabled: false, disabled_reason: "network policy is 'never'"` で列挙
    - 例: `typst_install`, `docs_sync`, `refs_fetch`, `refs_touch`

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

#### Phase 4: ビルドシステム（Week 4-5）

1. **build コマンド**
    - generate 統合
    - typst compile 実行
    - state.json 更新

2. **watch コマンド**
    - typstlab-watch 実装
    - notify crate 統合
    - debounce

#### Phase 5: その他（Week 5-6）

1. **link コマンド**
    - typst link 実装
    - uv link 実装
    - shim 生成

2. **refs コマンド**
    - DOI 取得（CrossRef API）
    - URL 取得（スクレイピング）
    - library.bib 更新

3. **sync コマンド**
    - link 統合
    - generate 統合
    - doctor actions 実行

4. **MCP サーバ**
    - typstlab-mcp 実装
    - tools 提供
    - --offline モード

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
