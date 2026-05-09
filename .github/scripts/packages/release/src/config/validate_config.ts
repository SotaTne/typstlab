import type { ReleaseConfig, ValidateReleaseConfigResult } from "./struct.ts";

export function validateReleaseConfig(config: ReleaseConfig): ValidateReleaseConfigResult {
  const errors: string[] = [];

  validateRequiredText(config.changelogPath, "CHANGELOG_PATH", errors);
  validateRequiredText(config.changelogTitle, "CHANGELOG_TITLE", errors);
  validateRequiredText(config.changelogHeader, "CHANGELOG_HEADER", errors);
  validateRequiredText(config.releaseNotesHeading, "RELEASE_NOTES_HEADING", errors);
  validateRequiredText(config.releaseBranchPattern, "RELEASE_BRANCH_PATTERN", errors);
  validateRequiredText(config.releaseTagPattern, "RELEASE_TAG_PATTERN", errors);
  validateRequiredText(config.releasePrTitlePattern, "RELEASE_PR_TITLE_PATTERN", errors);
  validateRequiredText(config.releasePrLabel, "RELEASE_PR_LABEL", errors);

  if (config.allowedCategories.length === 0) {
    errors.push("ALLOWED_CATEGORIES must not be empty");
  }

  validatePattern(config.releaseBranchPattern, "RELEASE_BRANCH_PATTERN", errors);
  validatePattern(config.releaseTagPattern, "RELEASE_TAG_PATTERN", errors);
  validatePattern(config.releasePrTitlePattern, "RELEASE_PR_TITLE_PATTERN", errors);

  if (errors.length > 0) {
    return { kind: "invalid", errors };
  }

  return { kind: "valid", config };
}

export function parseReleaseDraft(value: string | undefined): boolean | null {
  if (value === undefined || value.length === 0) {
    return true;
  }

  if (value === "true") {
    return true;
  }

  if (value === "false") {
    return false;
  }

  return null;
}

export function parseAllowedCategories(value: string | undefined, fallback: readonly string[]): string[] {
  if (value === undefined || value.trim().length === 0) {
    return [...fallback];
  }

  return value
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

export function countVersionPlaceholders(pattern: string): number {
  return pattern.split("{version}").length - 1;
}

export function expandVersionPattern(pattern: string, version: string): string {
  return pattern.split("{version}").join(version);
}

function validateRequiredText(value: string, name: string, errors: string[]): void {
  if (value.trim().length === 0) {
    errors.push(`${name} must not be empty`);
  }
}

function validatePattern(pattern: string, name: string, errors: string[]): void {
  const count = countVersionPlaceholders(pattern);
  if (count !== 1) {
    errors.push(`${name} must contain "{version}" exactly once`);
  }
}
