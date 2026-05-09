import { describe, expect, test } from "bun:test";
import {
  countVersionPlaceholders,
  expandVersionPattern,
  parseAllowedCategories,
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

describe("parseAllowedCategories", () => {
  test("splits comma separated categories and trims whitespace", () => {
    expect(parseAllowedCategories("Added, Changed ,Fixed", ["Other"])).toEqual([
      "Added",
      "Changed",
      "Fixed",
    ]);
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
        allowedCategories: ["Added", "Other"],
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
        allowedCategories: ["Added", "Other"],
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
        allowedCategories: [],
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
        "ALLOWED_CATEGORIES must not be empty",
        'RELEASE_BRANCH_PATTERN must contain "{version}" exactly once',
        'RELEASE_TAG_PATTERN must contain "{version}" exactly once',
      ],
    });
  });
});
