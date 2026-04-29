import type { RestEndpointMethodTypes } from "@octokit/plugin-rest-endpoint-methods";
import { extractVersion } from "../utils/version_extractor";

export type GitHubReleases = RestEndpointMethodTypes["repos"]["listReleases"]["response"]["data"];

export interface ConsistencyResult {
  missingInSchema: string[];
  extraInSchema: string[];
  missingInRequired: string[];
  extraInRequired: string[];
}

/**
 * Checks consistency between GitHub releases and the schema file.
 * @param schema The parsed JSON content of typst_version_schema.json
 * @param githubReleases List of releases from GitHub API (typst/typst)
 */
export function checkTypstSchemaConsistency(
  schema: any,
  githubReleases: GitHubReleases
): ConsistencyResult {

  if (!schema.hasOwnProperty("properties") || typeof schema.properties !== "object") {
    throw new Error("Invalid schema: 'properties' not found or not an object");
  }

  if (!schema.hasOwnProperty("required") || !Array.isArray(schema.required)) {
    throw new Error("Invalid schema: 'required' not found or not an array");
  }

  // 1. GitHub のタグから安定版（X.Y.Z）だけを抽出
  const githubVersions = githubReleases
    .map((r) => extractVersion(r.tag_name, "v{version}"))
    .filter((v): v is string => v !== null && /^\d+\.\d+\.\d+$/.test(v));

  // 2. スキーマの properties からバージョン形式のキーを抽出
  const properties = schema.properties || {};
  const schemaVersions = Object.keys(properties).filter((key) =>
    /^\d+\.\d+\.\d+$/.test(key)
  );

  // 3. スキーマの required リストを取得
  const requiredList = schema.required || [];

  // 4. 比較
  const missingInSchema = githubVersions.filter((v) => !schemaVersions.includes(v));
  const extraInSchema = schemaVersions.filter((v) => !githubVersions.includes(v));
  const missingInRequired = schemaVersions.filter((v) => !requiredList.includes(v));
  const extraInRequired = requiredList.filter((key: string) =>
    /^\d+\.\d+\.\d+$/.test(key) && !schemaVersions.includes(key)
  );

  return {
    missingInSchema,
    extraInSchema,
    missingInRequired,
    extraInRequired,
  };
}
