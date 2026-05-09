import type { ReleaseConfig, ValidateReleaseConfigResult } from "./struct.ts";
import {
  parseAllowedCategories,
  parseFallbackCategory,
  parseReleaseDraft,
  validateReleaseConfig,
} from "./validate_config.ts";

const DEFAULT_CHANGELOG_HEADER = [
  "All notable changes to this project will be documented in this file.",
  "",
  "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),",
  "and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).",
].join("\n");

export function loadReleaseConfig(env: NodeJS.ProcessEnv = process.env): ValidateReleaseConfigResult {
  const draft = parseReleaseDraft(env.RELEASE_DRAFT);
  if (draft === null) {
    return {
      kind: "invalid",
      errors: ["RELEASE_DRAFT must be \"true\" or \"false\""],
    };
  }

  const allowedCategories = parseAllowedCategories(env.ALLOWED_CATEGORIES);
  if (allowedCategories === undefined) {
    return {
      kind: "invalid",
      errors: ["ALLOWED_CATEGORIES must be a comma-separated list of known release note categories without Other"],
    };
  }

  const fallbackCategory = parseFallbackCategory(env.FALLBACK_CATEGORY);
  if (fallbackCategory === undefined) {
    return {
      kind: "invalid",
      errors: ["FALLBACK_CATEGORY must be Other or \"null\""],
    };
  }

  const config: ReleaseConfig = {
    changelogPath: env.CHANGELOG_PATH ?? "CHANGELOG.md",
    changelogTitle: env.CHANGELOG_TITLE ?? "Changelog",
    changelogHeader: env.CHANGELOG_HEADER ?? DEFAULT_CHANGELOG_HEADER,
    releaseNotesHeading: env.RELEASE_NOTES_HEADING ?? "Release Notes",
    allowedCategories,
    fallbackCategory,
    releaseBranchPattern: env.RELEASE_BRANCH_PATTERN ?? "release/v{version}",
    releaseTagPattern: env.RELEASE_TAG_PATTERN ?? "v{version}",
    releasePrTitlePattern: env.RELEASE_PR_TITLE_PATTERN ?? "release: v{version}",
    releasePrLabel: env.RELEASE_PR_LABEL ?? "release-pr",
    releaseDraft: draft,
  };

  return validateReleaseConfig(config);
}
