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
