import { describe, expect, test } from "bun:test";
import { DEFAULT_RELEASE_NOTE_CATEGORIES, type AllowedReleaseNoteCategory } from "../changelog/struct.ts";
import {
  countVersionPlaceholders,
  expandVersionPattern,
  parseAllowedCategories,
  parseFallbackCategory,
  parseReleaseDraft,
  validateReleaseConfig,
} from "./validate_config.ts";

describe("parseReleaseDraft", () => {
  test("accepts true and false", () => {
    expect(parseReleaseDraft("true")).toBe(true);
    expect(parseReleaseDraft("false")).toBe(false);
    expect(parseReleaseDraft(undefined)).toBe(true);
  });

  test("rejects invalid values", () => {
    expect(parseReleaseDraft("yes")).toBeNull();
  });
});

describe("parseFallbackCategory", () => {
  test("defaults to Other when env is empty", () => {
    expect(parseFallbackCategory(undefined)).toBe("Other");
  });

  test("accepts explicit null", () => {
    expect(parseFallbackCategory("null")).toBeNull();
  });

  test("accepts Other", () => {
    expect(parseFallbackCategory("Other")).toBe("Other");
  });

  test("rejects non-fallback categories", () => {
    expect(parseFallbackCategory("Added")).toBeUndefined();
    expect(parseFallbackCategory("Performance")).toBeUndefined();
  });
});

describe("parseAllowedCategories", () => {
  test("defaults to explicit categories when env is empty", () => {
    expect(parseAllowedCategories(undefined)).toEqual(DEFAULT_RELEASE_NOTE_CATEGORIES);
  });

  test("accepts comma separated categories", () => {
    expect(parseAllowedCategories("Added, Fixed,Changed")).toEqual([
      "Added",
      "Fixed",
      "Changed",
    ]);
  });

  test("rejects Other in allowed categories", () => {
    expect(parseAllowedCategories("Added,Other")).toBeUndefined();
  });
});

describe("countVersionPlaceholders", () => {
  test("counts version placeholders", () => {
    expect(countVersionPlaceholders("release/v{version}")).toBe(1);
    expect(countVersionPlaceholders("v{version}-pre-{version}")).toBe(2);
  });
});

describe("expandVersionPattern", () => {
  test("expands version placeholders", () => {
    expect(expandVersionPattern("release/v{version}", "1.2.3")).toBe("release/v1.2.3");
  });
});

describe("validateReleaseConfig", () => {
  test("accepts a complete configuration", () => {
    expect(
      validateReleaseConfig({
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader: "Header",
        releaseNotesHeading: "Release Notes",
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
        fallbackCategory: "Other",
        releaseBranchPattern: "release/v{version}",
        releaseTagPattern: "v{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "release-pr",
        releaseDraft: true,
      }),
    ).toEqual({
      kind: "valid",
      config: {
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader: "Header",
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

  test("rejects invalid patterns and empty required values", () => {
    expect(
      validateReleaseConfig({
        changelogPath: "",
        changelogTitle: "",
        changelogHeader: "",
        releaseNotesHeading: "",
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
        fallbackCategory: "Other",
        releaseBranchPattern: "release/v",
        releaseTagPattern: "v{version}-{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "",
        releaseDraft: true,
      }),
    ).toEqual({
      kind: "invalid",
      errors: [
        "CHANGELOG_PATH must not be empty",
        "CHANGELOG_TITLE must not be empty",
        "CHANGELOG_HEADER must not be empty",
        "RELEASE_NOTES_HEADING must not be empty",
        "RELEASE_PR_LABEL must not be empty",
        'RELEASE_BRANCH_PATTERN must contain "{version}" exactly once',
        'RELEASE_TAG_PATTERN must contain "{version}" exactly once',
      ],
    });
  });

  test("accepts configurations with null fallback category", () => {
    expect(
      validateReleaseConfig({
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader: "Header",
        releaseNotesHeading: "Release Notes",
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
        fallbackCategory: null,
        releaseBranchPattern: "release/v{version}",
        releaseTagPattern: "v{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "release-pr",
        releaseDraft: true,
      }),
    ).toEqual({
      kind: "valid",
      config: expect.objectContaining({
        fallbackCategory: null,
      }),
    });
  });

  test("rejects fallback categories other than Other or null", () => {
    expect(
      validateReleaseConfig({
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader: "Header",
        releaseNotesHeading: "Release Notes",
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
        fallbackCategory: "Added" as never,
        releaseBranchPattern: "release/v{version}",
        releaseTagPattern: "v{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "release-pr",
        releaseDraft: true,
      }),
    ).toEqual({
      kind: "invalid",
      errors: ["FALLBACK_CATEGORY must be Other or null"],
    });
  });

  test("rejects allowed categories that include Other", () => {
    expect(
      validateReleaseConfig({
        changelogPath: "CHANGELOG.md",
        changelogTitle: "Changelog",
        changelogHeader: "Header",
        releaseNotesHeading: "Release Notes",
        allowedCategories:
          [...DEFAULT_RELEASE_NOTE_CATEGORIES, "Other" as const] as unknown as readonly AllowedReleaseNoteCategory[],
        fallbackCategory: "Other",
        releaseBranchPattern: "release/v{version}",
        releaseTagPattern: "v{version}",
        releasePrTitlePattern: "release: v{version}",
        releasePrLabel: "release-pr",
        releaseDraft: true,
      }),
    ).toEqual({
      kind: "invalid",
      errors: ["ALLOWED_CATEGORIES must not include Other"],
    });
  });
});
