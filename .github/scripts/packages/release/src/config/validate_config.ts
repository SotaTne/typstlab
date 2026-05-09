import {
  DEFAULT_RELEASE_NOTE_CATEGORIES,
  type AllowedReleaseNoteCategory,
} from "../changelog/struct.ts";
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
  validateAllowedCategories(config.allowedCategories, errors);
  validateFallbackCategory(config.fallbackCategory, errors);

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

export function parseFallbackCategory(value: string | undefined): "Other" | null | undefined {
  if (value === undefined || value.trim().length === 0) {
    return "Other";
  }

  if (value === "null") {
    return null;
  }

  if (value === "Other") {
    return value;
  }

  return undefined;
}

export function parseAllowedCategories(
  value: string | undefined,
): readonly AllowedReleaseNoteCategory[] | undefined {
  if (value === undefined || value.trim().length === 0) {
    return DEFAULT_RELEASE_NOTE_CATEGORIES;
  }

  const categories = value
    .split(",")
    .map((category) => category.trim())
    .filter((category) => category.length > 0);

  if (categories.length === 0) {
    return undefined;
  }

  const parsed: AllowedReleaseNoteCategory[] = [];
  for (const category of categories) {
    if (!isAllowedReleaseNoteCategory(category)) {
      return undefined;
    }

    if (!parsed.includes(category)) {
      parsed.push(category);
    }
  }

  return parsed;
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

function validateAllowedCategories(
  allowedCategories: readonly AllowedReleaseNoteCategory[],
  errors: string[],
): void {
  if (allowedCategories.length === 0) {
    errors.push("ALLOWED_CATEGORIES must not be empty");
    return;
  }

  if ((allowedCategories as readonly string[]).includes("Other")) {
    errors.push("ALLOWED_CATEGORIES must not include Other");
    return;
  }

  for (const category of allowedCategories) {
    if (!isAllowedReleaseNoteCategory(category)) {
      errors.push("ALLOWED_CATEGORIES must contain only known release note categories");
      return;
    }
  }
}

function validateFallbackCategory(fallbackCategory: "Other" | null, errors: string[]): void {
  if (fallbackCategory === null || fallbackCategory === "Other") {
    return;
  }

  errors.push("FALLBACK_CATEGORY must be Other or null");
}

function isAllowedReleaseNoteCategory(value: string): value is AllowedReleaseNoteCategory {
  return (DEFAULT_RELEASE_NOTE_CATEGORIES as readonly string[]).includes(value);
}
