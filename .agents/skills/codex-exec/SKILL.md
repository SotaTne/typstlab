---
name: codex-exec
description: >
  範囲が限定されたコーディング、リファクタリング、デバッグ、コードレビューを
  ローカルの OpenAI Codex CLI（`codex exec`）へ委譲し、結果を検証する。
  Use when the user asks to run Codex, get a second opinion from Codex, run a Codex review,
  or offload a self-contained implementation or review task.
---

# Codex CLI への委譲（`codex exec`）

ローカルの `codex` CLI（`codex-cli 0.169.0+`）を非対話モードで呼び出して自己完結した実装・デバッグ・コードレビューを委譲し、ユーザーへ報告する前に出力を必ず検証する。

## 前提条件

タスク実行前に、`codex` がインストール済みかつ認証済みであることを確認する:

```bash
command -v codex && codex --version && codex login status
```

`codex` が見つからない、または未認証の場合は処理を止め、ユーザーに `@openai/codex` のインストールと `codex login` の実行を案内する（設定や実行環境の診断には `codex doctor` を使う）。

## サンドボックスと安全ルール

- **基本は読み取り専用**（`--sandbox read-only`）をデフォルトとする（コードレビュー、解析、デバッグ、設計・計画など）。
- **`workspace-write` を使うのはファイル変更が明示的に必要なときだけ**とし、`--sandbox workspace-write`（または `workspace-write` サンドボックス内で自動承認レビューを使う場合は `--approve-for-me`）を指定する。
- **`--full-auto` は使わない**（`codex-cli 0.169.0` の `codex exec` では未サポートのためエラーになる）。
- **`--dangerously-bypass-approvals-and-sandbox`、`--dangerously-bypass-hook-trust`、`--sandbox danger-full-access` は絶対に指定しない**（ユーザーが明示的に指示した場合のみ例外）。
- **プロンプトは必ず stdin（`-`）経由で渡す**。シェルのクォート事故やオープン状態の stdin によるハングを防ぐため、クォート付きヒアドキュメント（`<<'EOF'`）を使う。

## 実行パターン

### 1. 読み取り専用のレビュー・解析

セッション履歴を残さない単発実行には `--ephemeral` を付ける。後続で `resume` する予定がある場合は `--ephemeral` を外す:

```bash
codex exec \
  --sandbox read-only \
  --ephemeral \
  --skip-git-repo-check \
  --color never \
  -C "<TARGET_DIR>" \
  - <<'EOF'
<CLEAR, SELF-CONTAINED PROMPT WITH TARGET FILES AND CONSTRAINTS>
EOF
```

### 2. リポジトリの標準コードレビュー

`-C "<TARGET_DIR>"` など `exec` 側のオプションは `review` の前に置き、比較対象（`--uncommitted`、`--base <BRANCH>`、または `--commit <SHA>`＋任意の `--title <TITLE>`）を指定するか、stdin でカスタム指示を渡す:

```bash
codex exec \
  -C "<TARGET_DIR>" \
  review --uncommitted
```

```bash
codex exec \
  -C "<TARGET_DIR>" \
  review --base main - <<'EOF'
<OPTIONAL FOCUS AREAS OR CONSTRAINTS>
EOF
```

### 3. ファイル編集を伴う実装タスク

```bash
codex exec \
  --sandbox workspace-write \
  --skip-git-repo-check \
  -C "<TARGET_DIR>" \
  - <<'EOF'
<TASK BRIEF: GOAL, ALLOWED FILES, CONSTRAINTS, AND VERIFICATION COMMAND>
EOF
```

- 承認リクエストを `workspace-write` サンドボックス内の自動承認レビューへ回したいときは、`--sandbox workspace-write` の代わりに `--approve-for-me` を使う。
- 変更を独立した管理下の Git worktree に隔離したい場合は `--worktree` を、作業ルート以外の書き込み先も必要な場合は `--add-dir <DIR>` を追加する。

### 4. 既存セッションの再開・分岐

`-C "<TARGET_DIR>"` などの `exec` 側フラグは `exec` と `resume` または `fork` の間に置く（`--ephemeral` 付きで作成したセッションは再開できない点に注意）:

```bash
codex exec -C "<TARGET_DIR>" resume --last --skip-git-repo-check - <<'EOF'
<FOLLOW-UP INSTRUCTION>
EOF
```

- `--last` の代わりに `<SESSION_ID>` で特定セッションを指定でき、`--all` を付けると `cwd` による絞り込みを外せる。
- 元の履歴を変更せずに別方向を試すときは `fork` を使う:

```bash
codex exec -C "<TARGET_DIR>" fork <SESSION_ID> --skip-git-repo-check - <<'EOF'
<BRANCHING INSTRUCTION>
EOF
```

## オプションフラグ

- モデル指定はユーザーが求めたときのみ: `-m, --model <MODEL>`
- 設定キーの上書き（例: 推論レベル）: `-c model_reasoning_effort="<low|medium|high|xhigh>"`（別名 `--config`）
- `$CODEX_HOME` 上のプロファイルを重ねる: `-p, --profile <NAME>`
- エージェントの最終応答だけをファイルへ書き出す（エラー確認のため `2>/dev/null` より推奨）: `-o, --output-last-message <FILE>`
- 構造化出力／スキーマ制約: `--json`（stdout に JSONL イベント出力）、`--output-schema <FILE>`
- 出力・ワークスペース補助: `--color <always|never|auto>`、`--ephemeral`、`--worktree`、`--add-dir <DIR>`、`-i, --image <FILE>...`
- stdout だけを取り出して stderr の進捗・思考ログを捨てたい場合のみ末尾に `2>/dev/null` を付ける

## 実行後の検証

1. 編集タスクでは `git status` と `git diff`（`--worktree` 使用時は作成された worktree 側も）を確認し、依頼範囲外の変更がないか確かめる。
2. Codex 側で未実施の場合は、関連テスト・フォーマッタ・リンターを自分で実行して確認する。
3. Codex の結果、変更されたファイル、検証結果を簡潔にまとめてユーザーへ報告する。
