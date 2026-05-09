import type {
  GuardChangelogUpdateOptions,
  GuardChangelogUpdateResult,
} from "./struct.ts";

export function guardChangelogUpdate(
  options: GuardChangelogUpdateOptions,
): GuardChangelogUpdateResult {
  if (!hasChangelogPathChange(options.changedPaths, options.changelogPath)) {
    return { kind: "allowed" };
  }

  if (options.isReleasePr) {
    return { kind: "allowed" };
  }

  return {
    kind: "blocked",
    reason: `${options.changelogPath} can only be updated by release PRs`,
  };
}

export function hasChangelogPathChange(
  changedPaths: readonly string[],
  changelogPath: string,
): boolean {
  return changedPaths.includes(changelogPath);
}
