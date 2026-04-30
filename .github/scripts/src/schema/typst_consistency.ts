import type { RestEndpointMethodTypes } from "@octokit/plugin-rest-endpoint-methods";
import { extractVersion } from "../utils/version_extractor";

export type GitHubReleases = RestEndpointMethodTypes["repos"]["listReleases"]["response"]["data"];

export interface ConsistencyResult {
  missingInSchema: string[];
  extraInSchema: string[];
  missingInRequired: string[];
  extraInRequired: string[];
  ignoredInProperties: string[];
  ignoredInRequired: string[];
  effectiveGithubVersions: string[];
  effectiveSchemaVersions: string[];
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

  const ignoredVersions = Array.isArray(schema.version_ignores)
    ? schema.version_ignores.filter((version: unknown): version is string =>
        typeof version === "string" && /^\d+\.\d+\.\d+$/.test(version)
      )
    : [];

  const ignoredVersionSet = new Set(ignoredVersions);

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

  const ignoredInProperties = schemaVersions.filter((v) => ignoredVersionSet.has(v));
  const ignoredInRequired = requiredList.filter(
    (key: string) => /^\d+\.\d+\.\d+$/.test(key) && ignoredVersionSet.has(key)
  );

  const effectiveGithubVersions = githubVersions.filter((v) => !ignoredVersionSet.has(v));
  const effectiveSchemaVersions = schemaVersions.filter((v) => !ignoredVersionSet.has(v));
  const effectiveRequiredList = requiredList.filter((key: string) => !ignoredVersionSet.has(key));

  // 4. 比較
  const missingInSchema = effectiveGithubVersions.filter((v) => !effectiveSchemaVersions.includes(v));
  const extraInSchema = effectiveSchemaVersions.filter((v) => !effectiveGithubVersions.includes(v));
  const missingInRequired = effectiveSchemaVersions.filter((v) => !effectiveRequiredList.includes(v));
  const extraInRequired = requiredList.filter((key: string) =>
    /^\d+\.\d+\.\d+$/.test(key) && !effectiveSchemaVersions.includes(key) && !ignoredVersionSet.has(key)
  );

  return {
    missingInSchema,
    extraInSchema,
    missingInRequired,
    extraInRequired,
    ignoredInProperties,
    ignoredInRequired,
    effectiveGithubVersions,
    effectiveSchemaVersions,
  };
}
