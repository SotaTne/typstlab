import type { AsyncFunctionArguments } from "@actions/github-script";
import { extractReleaseNotesSection } from "../changelog/extract_release_notes.ts";
import { guardChangelogUpdate } from "../changelog/guard_changelog_update.ts";
import type { ReleaseConfig } from "../config/struct.ts";
import { validateReleaseNotes } from "../changelog/validate_release_notes.ts";
import { loadPullRequestLikeFromActionArgs, loadPullRequestLikeFromGitHub } from "../github/collect_pull_requests.ts";
import type { GitHubClientLike, GitHubContextLike, PullRequestLike } from "../github/struct.ts";

export type PrCheckFinding = {
  severity: "warning" | "failure";
  message: string;
};

export type PrCheckResult =
  | {
      kind: "pass";
      findings: PrCheckFinding[];
      isReleaseCandidate: boolean;
    }
  | {
      kind: "warning";
      findings: PrCheckFinding[];
      isReleaseCandidate: boolean;
    }
  | {
      kind: "failure";
      findings: PrCheckFinding[];
      isReleaseCandidate: boolean;
    };

export function analyzePullRequestForRelease(
  config: ReleaseConfig,
  pullRequest: PullRequestLike,
): PrCheckResult {
  const findings: PrCheckFinding[] = [];
  const isReleaseCandidate = isReleasePr(config, pullRequest);
  const releaseNotesSection = extractReleaseNotesSection(pullRequest.body, config.releaseNotesHeading);

  if (releaseNotesSection.kind === "missing") {
    findings.push({
      severity: isReleaseCandidate ? "failure" : "warning",
      message: `missing ## ${config.releaseNotesHeading} section`,
    });
  } else {
    const releaseNotes = validateReleaseNotes(releaseNotesSection.content, {
      allowedCategories: config.allowedCategories,
      fallbackCategory: config.fallbackCategory,
    });

    if (releaseNotes.kind === "invalid") {
      for (const error of releaseNotes.errors) {
        findings.push({ severity: "failure", message: error });
      }
    }
  }

  const changelogGuard = guardChangelogUpdate({
    changedPaths: pullRequest.changedPaths,
    changelogPath: config.changelogPath,
    isReleasePr: isReleaseCandidate,
  });

  if (changelogGuard.kind === "blocked") {
    findings.push({
      severity: "warning",
      message: changelogGuard.reason,
    });
  }

  const hasFailure = findings.some((finding) => finding.severity === "failure");
  const hasWarning = findings.some((finding) => finding.severity === "warning");

  if (hasFailure) {
    return { kind: "failure", findings, isReleaseCandidate };
  }

  if (hasWarning) {
    return { kind: "warning", findings, isReleaseCandidate };
  }

  return { kind: "pass", findings, isReleaseCandidate };
}

export async function analyzePullRequestForReleaseFromGitHub(
  config: ReleaseConfig,
  github: GitHubClientLike,
  context: GitHubContextLike,
  pullNumber = context.payload.pull_request?.number,
): Promise<PrCheckResult> {
  const pullRequest = await loadPullRequestLikeFromGitHub(github, context, pullNumber);
  return analyzePullRequestForRelease(config, pullRequest);
}

export async function analyzePullRequestForReleaseFromActionArgs(
  config: ReleaseConfig,
  args: Pick<AsyncFunctionArguments, "github" | "context">,
  pullNumber = args.context.payload.pull_request?.number,
): Promise<PrCheckResult> {
  const pullRequest = await loadPullRequestLikeFromActionArgs(args, pullNumber);
  return analyzePullRequestForRelease(config, pullRequest);
}

export function isReleasePr(
  config: Pick<ReleaseConfig, "releaseBranchPattern" | "releasePrTitlePattern" | "releasePrLabel">,
  pullRequest: Pick<PullRequestLike, "headRefName" | "title" | "labels">,
): boolean {
  return (
    matchesPattern(pullRequest.headRefName, config.releaseBranchPattern) &&
    matchesPattern(pullRequest.title, config.releasePrTitlePattern) &&
    pullRequest.labels.includes(config.releasePrLabel)
  );
}

function matchesPattern(value: string, pattern: string): boolean {
  const escaped = pattern
    .split("{version}")
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("(.+)");
  return new RegExp(`^${escaped}$`).test(value);
}
