import { loadReleaseConfig } from "./config/load_config.ts";
import type { ReleaseConfig, ValidateReleaseConfigResult } from "./config/struct.ts";
import { analyzePullRequestForRelease, analyzePullRequestForReleaseFromActionArgs } from "./commands/pr_check.ts";
import type { PrCheckResult } from "./commands/pr_check.ts";
import type { AsyncFunctionArguments } from "@actions/github-script";
import { normalizeVersion } from "./version/version_validator.ts";

export type { ReleaseConfig, ValidateReleaseConfigResult, PrCheckResult };

export { loadReleaseConfig };

export function validateVersion(version: string): string {
  return normalizeVersion(version);
}

export function runPrCheckFromJson(
  config: ReleaseConfig,
  pullRequestJson: string,
): PrCheckResult {
  const pullRequest = JSON.parse(pullRequestJson) as Parameters<typeof analyzePullRequestForRelease>[1];
  return analyzePullRequestForRelease(config, pullRequest);
}

export async function runPrCheckFromActionArgs(
  config: ReleaseConfig,
  args: Pick<AsyncFunctionArguments, "github" | "context">,
): Promise<PrCheckResult> {
  return analyzePullRequestForReleaseFromActionArgs(config, args);
}
