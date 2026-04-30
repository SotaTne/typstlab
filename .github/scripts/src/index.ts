import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import type { AsyncFunctionArguments } from "@actions/github-script";
import { checkJsonFileKeys } from "./schema/json_key_checker";
import { checkTypstSchemaConsistency } from "./schema/typst_consistency";
import { reportSchemaInconsistency } from "./app/schema_issue_reporter";

/**
 * Job: Typst スキーマの整合性チェックとレポート
 */
export async function jobCheckTypstSchemaConsistency(args: AsyncFunctionArguments) {
  const { github, core } = args;
  const schemaRelativePath = "crates/typstlab-base/src/version_resolver_jsons/typst_version_schema.json";

  // 1. スキーマファイルの調達
  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const workspaceRoot = process.env.GITHUB_WORKSPACE || path.resolve(__dirname, "../../../");

  const schemaPath = path.join(
    workspaceRoot,
    schemaRelativePath
  );
  
  if (!fs.existsSync(schemaPath)) {
    core.setFailed(`Schema file not found at: ${schemaPath}`);
    return;
  }

  const schema = JSON.parse(fs.readFileSync(schemaPath, "utf-8"));

  // 2. GitHub リリースの調達
  core.info("Fetching Typst releases from GitHub...");
  const releases = await github.paginate(github.rest.repos.listReleases, {
    owner: "typst",
    repo: "typst",
    per_page: 100,
  });

  // 3. 整合性チェックの実行
  const result = checkTypstSchemaConsistency(schema, releases);

  // 4. 他 JSON ファイルのキーチェック（副作用チェック）
  const resolverDir = path.join(workspaceRoot, "crates/typstlab-base/src/version_resolver_jsons");
  const keyCheckResult = checkJsonFileKeys(resolverDir, workspaceRoot, result.effectiveGithubVersions);

  // 5. 報告（Issue作成）を専門家に委譲
  await reportSchemaInconsistency(args, result, keyCheckResult, schemaRelativePath);

  // 監視ジョブとしては成功終了にする。異常は issue と warning で伝える。
  if (
    result.missingInSchema.length > 0 ||
    result.extraInSchema.length > 0 ||
    result.missingInRequired.length > 0 ||
    result.extraInRequired.length > 0 ||
    result.ignoredInProperties.length > 0 ||
    result.ignoredInRequired.length > 0 ||
    keyCheckResult.files.length > 0
  ) {
    core.warning("Schema is inconsistent with GitHub releases. Check the created issue for details.");
  }
}
