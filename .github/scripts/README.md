# typstlab-scripts

GitHub Actions から呼び出す TypeScript スクリプト群です。

## Setup

```bash
bun install
```

## Test

```bash
bun test
```

## Build

Actions から読み込む `dist/index.js` を作ります。

```bash
bun run build:index
```

## JSON Check

`typst_version_schema.json` を 2 段階で検証します。
1 段目は schema 自体を `--strict=false` 付きで compile し、`version_ignores` のような独自キーを許容します。
2 段目は残りの JSON を schema で validate します。`version_ignores` は no-op keyword として読み込まれます。

```bash
bun run json-check:schema
bun run json-check:files
```

まとめて実行する場合は:

```bash
bun run json-check
```

## Local check

`actions/github-script` から呼ばれる本体は `src/index.ts` の
`jobCheckTypstSchemaConsistency` です。

```bash
bun run build:index
node --input-type=module -e "import('./dist/index.js').then((m) => console.log(Object.keys(m)))"
```

## CLI

ローカルで手動実行する場合は `src/cli.ts` を使います。

```bash
bun run cli list
bun run cli run jobCheckTypstSchemaConsistency
```
