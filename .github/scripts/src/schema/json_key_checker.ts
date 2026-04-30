import * as fs from "node:fs";
import * as path from "node:path";

export interface JsonFileKeyResult {
  filePath: string;
  missingKeys: string[];
  extraKeys: string[];
}

export interface JsonKeyCheckResult {
  files: JsonFileKeyResult[];
}

const VERSION_KEY_PATTERN = /^\d+\.\d+\.\d+$/;

function normalizePath(workspaceRoot: string, filePath: string): string {
  return path.relative(workspaceRoot, filePath).split(path.sep).join("/");
}

function readJsonFile(filePath: string): Record<string, unknown> {
  const content = fs.readFileSync(filePath, "utf-8");
  const parsed = JSON.parse(content);

  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`Invalid JSON object in ${filePath}`);
  }

  return parsed as Record<string, unknown>;
}

function getVersionKeys(value: Record<string, unknown>): string[] {
  return Object.keys(value).filter((key) => VERSION_KEY_PATTERN.test(key));
}

export function checkJsonFileKeys(
  resolverDir: string,
  workspaceRoot: string,
  effectiveVersions: string[]
): JsonKeyCheckResult {
  const schemaFileName = "typst_version_schema.json";
  const effectiveVersionSet = new Set(effectiveVersions);
  const fileNames = fs
    .readdirSync(resolverDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".json") && entry.name !== schemaFileName)
    .map((entry) => entry.name)
    .sort();

  const files = fileNames.flatMap((fileName) => {
    const absFilePath = path.join(resolverDir, fileName);
    const fileJson = readJsonFile(absFilePath);
    const fileVersionKeys = getVersionKeys(fileJson);
    const missingKeys = effectiveVersions.filter((version) => !fileVersionKeys.includes(version));
    const extraKeys = fileVersionKeys.filter((version) => !effectiveVersionSet.has(version));

    if (missingKeys.length === 0 && extraKeys.length === 0) {
      return [];
    }

    return [{
      filePath: normalizePath(workspaceRoot, absFilePath),
      missingKeys,
      extraKeys,
    }];
  });

  return { files };
}
