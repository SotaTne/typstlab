import { describe, expect, test } from "bun:test";
import { DEFAULT_RELEASE_NOTE_CATEGORIES } from "../changelog/struct.ts";
import { loadReleaseConfig } from "./load_config.ts";

describe("loadReleaseConfig", () => {
  test("uses defaults when env is empty", () => {
    expect(loadReleaseConfig({} as NodeJS.ProcessEnv)).toEqual({
      kind: "valid",
      config: {
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader:
          "All notable changes to this project will be documented in this file.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\nand this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).",
        releaseNotesHeading: "Release Notes",
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
        fallbackCategory: "Other",
        releaseBranchPattern: "release/v{version}",
        releaseTagPattern: "v{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "release-pr",
        releaseDraft: true,
      },
    });
  });

  test("rejects invalid draft values", () => {
    expect(
      loadReleaseConfig({
        RELEASE_DRAFT: "maybe",
      } as NodeJS.ProcessEnv),
    ).toEqual({
      kind: "invalid",
      errors: ['RELEASE_DRAFT must be "true" or "false"'],
    });
  });

  test("accepts explicit allowed categories and null fallback category", () => {
    expect(
      loadReleaseConfig({
        ALLOWED_CATEGORIES: "Added,Changed,Fixed",
        FALLBACK_CATEGORY: "null",
      } as NodeJS.ProcessEnv),
    ).toEqual({
      kind: "valid",
      config: {
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader:
          "All notable changes to this project will be documented in this file.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\nand this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).",
        releaseNotesHeading: "Release Notes",
        allowedCategories: ["Added", "Changed", "Fixed"],
        fallbackCategory: null,
        releaseBranchPattern: "release/v{version}",
        releaseTagPattern: "v{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "release-pr",
        releaseDraft: true,
      },
    });
  });

  test("rejects Other in allowed categories", () => {
    expect(
      loadReleaseConfig({
        ALLOWED_CATEGORIES: "Added,Other",
      } as NodeJS.ProcessEnv),
    ).toEqual({
      kind: "invalid",
      errors: ["ALLOWED_CATEGORIES must be a comma-separated list of known release note categories without Other"],
    });
  });

  test("rejects unknown fallback categories", () => {
    expect(
      loadReleaseConfig({
        FALLBACK_CATEGORY: "Performance",
      } as NodeJS.ProcessEnv),
    ).toEqual({
      kind: "invalid",
      errors: ['FALLBACK_CATEGORY must be Other or "null"'],
    });
  });
});
