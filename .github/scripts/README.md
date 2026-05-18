# typstlab-github-scripts

GitHub Actions から呼び出す TypeScript スクリプト群です。Bun workspace として管理し、用途ごとに package を分けています。

## Packages

- `@typstlab/toolchain-monitor`: Typst toolchain / resolver JSON の監視と schema check

## Setup

```bash
bun install
```

## Test

```bash
bun run test
```

## Typecheck

```bash
bun run typecheck
```

## Build

Actions から読み込む package ごとの `dist/index.js` を作ります。

```bash
bun run build:index
bun run build:cli
```

## JSON Check

`typst_version_schema.json` を 2 段階で検証します。
1 段目は schema 自体を compile し、`version_ignores` を custom keyword として読み込みます。
2 段目は残りの JSON を schema で validate します。`version_ignores` は validate 側でも同じ keyword 定義で読み込まれます。

```bash
bun run json-check:schema
bun run json-check:files
```

まとめて実行する場合は:

```bash
bun run json-check
```

## Local check

`actions/github-script` から呼ばれる本体は `packages/toolchain-monitor/src/index.ts` の
`jobCheckTypstSchemaConsistency` です。

```bash
bun run build:index
node --input-type=module -e "import('./packages/toolchain-monitor/dist/index.js').then((m) => console.log(Object.keys(m)))"
```

## CLI

ローカルで手動実行する場合は package ごとの CLI を使います。

```bash
bun --filter '@typstlab/toolchain-monitor' run:cli list
bun --filter '@typstlab/toolchain-monitor' run:cli run jobCheckTypstSchemaConsistency
```
