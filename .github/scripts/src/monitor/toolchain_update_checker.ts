import * as fs from "node:fs";
import * as path from "node:path";
import type { AsyncFunctionArguments } from "@actions/github-script";
import { extractVersion } from "../utils/version_extractor";

const STABLE_VERSION_PATTERN = /^\d+\.\d+\.\d+$/;

export interface ToolchainVersionCheckResult {
  typstVersion: string;
  missingVersions: string[];
  extraVersions: string[];
  duplicateVersions: string[];
}

export interface ToolchainIgnoreCheckResult {
  extraVersions: string[];
  duplicateVersions: string[];
}

export interface ToolchainFileCheckResult {
  filePath: string;
  baseUrl: string;
  versionPattern: string;
  releaseVersions: string[];
  ignoredVersions: string[];
  versionChecks: ToolchainVersionCheckResult[];
  ignoreCheck: ToolchainIgnoreCheckResult;
}

export interface ToolchainUpdateResult {
  files: ToolchainFileCheckResult[];
}

type ResolverJson = Record<string, unknown>;
type ReleaseList = Array<{ tag_name?: string | null }>;

function normalizePath(workspaceRoot: string, filePath: string): string {
  return path.relative(workspaceRoot, filePath).split(path.sep).join("/");
}

function isResolverJson(value: unknown): value is ResolverJson {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function readJsonFile(filePath: string): ResolverJson {
  const content = fs.readFileSync(filePath, "utf-8");
  const parsed = JSON.parse(content);

  if (!isResolverJson(parsed)) {
    throw new Error(`Invalid JSON object in ${filePath}`);
  }

  return parsed;
}

function unique(values: string[]): string[] {
  return [...new Set(values)];
}

function findDuplicates(values: string[]): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();

  for (const value of values) {
    if (seen.has(value)) {
      duplicates.add(value);
      continue;
    }
    seen.add(value);
  }

  return [...duplicates];
}

function difference(left: string[], right: string[]): string[] {
  const rightSet = new Set(right);
  return left.filter((value) => !rightSet.has(value));
}

function parseGitHubRepo(baseUrl: string): { owner: string; repo: string } {
  const parsed = new URL(baseUrl);

  if (parsed.hostname !== "github.com") {
    throw new Error(`Invalid base_url: ${baseUrl}`);
  }

  const segments = parsed.pathname.split("/").filter(Boolean);
  if (segments.length !== 2) {
    throw new Error(`Invalid base_url path: ${baseUrl}`);
  }

  const owner = segments[0];
  const repo = segments[1];
  if (owner === undefined || repo === undefined) {
    throw new Error(`Invalid base_url path: ${baseUrl}`);
  }

  return { owner, repo };
}

function getReleaseVersions(releases: ReleaseList, versionPattern: string): string[] {
  const versions = releases
    .map((release) => {
      const tagName = release.tag_name;
      if (!tagName) {
        return null;
      }

      const version = extractVersion(tagName, versionPattern);
      if (version === null || !STABLE_VERSION_PATTERN.test(version)) {
        return null;
      }

      return version;
    })
    .filter((version): version is string => version !== null);

  return unique(versions);
}

function getVersionKeys(json: ResolverJson): Array<[string, string[]]> {
  return Object.entries(json)
    .filter((entry): entry is [string, unknown[]] => {
      const [key, value] = entry;
      return STABLE_VERSION_PATTERN.test(key) && Array.isArray(value);
    })
    .map(([key, value]) => [
      key,
      value.filter((entry: unknown): entry is string => typeof entry === "string")
    ] as [string, string[]]);
}

function getIgnoredVersions(json: ResolverJson): string[] {
  if (!Array.isArray(json.ignores)) {
    return [];
  }

  return json.ignores.filter((value: unknown): value is string => typeof value === "string" && STABLE_VERSION_PATTERN.test(value));
}

async function fetchReleaseVersions(
  github: AsyncFunctionArguments["github"],
  baseUrl: string,
  versionPattern: string
): Promise<string[]> {
  const { owner, repo } = parseGitHubRepo(baseUrl);
  const releases = await github.paginate(github.rest.repos.listReleases, {
    owner,
    repo,
    per_page: 100,
  }) as ReleaseList;

  return getReleaseVersions(releases, versionPattern);
}

function buildFileCheckResult(
  filePath: string,
  baseUrl: string,
  versionPattern: string,
  releaseVersions: string[],
  json: ResolverJson
): ToolchainFileCheckResult {
  const versionKeys = getVersionKeys(json);
  const ignoredVersions = unique(getIgnoredVersions(json));
  const ignoredVersionSet = new Set(ignoredVersions);
  const effectiveReleaseVersions = releaseVersions.filter((version) => !ignoredVersionSet.has(version));

  const versionChecks = versionKeys.map(([typstVersion, values]) => {
    const effectiveValues = values.filter((version) => !ignoredVersionSet.has(version));
    const uniqueEffectiveValues = unique(effectiveValues);
    return {
      typstVersion,
      missingVersions: difference(effectiveReleaseVersions, uniqueEffectiveValues),
      extraVersions: difference(uniqueEffectiveValues, effectiveReleaseVersions),
      duplicateVersions: findDuplicates(effectiveValues),
    };
  });

  const ignoreCheck = {
    extraVersions: difference(ignoredVersions, releaseVersions),
    duplicateVersions: findDuplicates(getIgnoredVersions(json)),
  };

  return {
    filePath,
    baseUrl,
    versionPattern,
    releaseVersions,
    ignoredVersions,
    versionChecks,
    ignoreCheck,
  };
}

export async function checkToolchainUpdate(
  args: AsyncFunctionArguments,
  resolverDir: string,
  workspaceRoot: string
): Promise<ToolchainUpdateResult> {
  const { github } = args;
  const schemaFileName = "typst_version_schema.json";
  const fileNames = fs
    .readdirSync(resolverDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".json") && entry.name !== schemaFileName)
    .map((entry) => entry.name)
    .sort();

  const files: ToolchainFileCheckResult[] = [];
  const releaseCache = new Map<string, string[]>();

  for (const fileName of fileNames) {
    const absFilePath = path.join(resolverDir, fileName);
    const json = readJsonFile(absFilePath);

    const baseUrl = json.base_url;
    const versionPattern = json.version_pattern;
    if (typeof baseUrl !== "string" || typeof versionPattern !== "string") {
      throw new Error(`Invalid resolver JSON metadata in ${absFilePath}`);
    }

    const cacheKey = `${baseUrl}::${versionPattern}`;
    const releaseVersions = releaseCache.has(cacheKey)
      ? releaseCache.get(cacheKey)!
      : await fetchReleaseVersions(github, baseUrl, versionPattern);

    releaseCache.set(cacheKey, releaseVersions);

    const fileCheckResult = buildFileCheckResult(
      normalizePath(workspaceRoot, absFilePath),
      baseUrl,
      versionPattern,
      releaseVersions,
      json
    );

    const hasVersionIssues = fileCheckResult.versionChecks.some(
      (check) => check.missingVersions.length > 0 || check.extraVersions.length > 0 || check.duplicateVersions.length > 0
    );
    const hasIgnoreIssues = fileCheckResult.ignoreCheck.extraVersions.length > 0 || fileCheckResult.ignoreCheck.duplicateVersions.length > 0;

    if (hasVersionIssues || hasIgnoreIssues) {
      files.push(fileCheckResult);
    }
  }

  return { files };
}
