import { describe, expect, test } from "bun:test";
import { DEFAULT_RELEASE_NOTE_CATEGORIES } from "../changelog/struct.ts";
import type { ReleaseConfig } from "../config/struct.ts";
import {
  analyzePullRequestForRelease,
  analyzePullRequestForReleaseFromGitHub,
  isReleasePr,
} from "./pr_check.ts";

const config: ReleaseConfig = {
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
};

describe("isReleasePr", () => {
  test("returns true only when branch, title, and label all match", () => {
    expect(
      isReleasePr(config, {
        headRefName: "release/v1.2.3",
        title: "release: v1.2.3",
        labels: ["release-pr"],
      }),
    ).toBe(true);
  });

  test("returns false when branch misses", () => {
    expect(
      isReleasePr(config, {
        headRefName: "feature/foo",
        title: "release: v1.2.3",
        labels: ["release-pr"],
      }),
    ).toBe(false);
  });

  test("returns false when title misses", () => {
    expect(
      isReleasePr(config, {
        headRefName: "release/v1.2.3",
        title: "something else",
        labels: ["release-pr"],
      }),
    ).toBe(false);
  });

  test("returns false when label misses", () => {
    expect(
      isReleasePr(config, {
        headRefName: "release/v1.2.3",
        title: "release: v1.2.3",
        labels: ["bug"],
      }),
    ).toBe(false);
  });
});

describe("analyzePullRequestForRelease", () => {
  test("passes when release notes are valid and changelog is untouched", () => {
    expect(
      analyzePullRequestForRelease(config, {
        body: [
          "## Summary",
          "",
          "Feature work.",
          "",
          "## Release Notes",
          "",
          "- Added: Support release automation.",
          "- Improve release diagnostics.",
        ].join("\n"),
        changedPaths: ["src/lib.ts"],
        headRefName: "feature/foo",
        title: "Add release automation",
        labels: [],
      }),
    ).toEqual({
      kind: "pass",
      findings: [],
      isReleaseCandidate: false,
    });
  });

  test("warns when release notes are missing on a normal PR", () => {
    expect(
      analyzePullRequestForRelease(config, {
        body: "## Summary\n\nFeature work.",
        changedPaths: ["src/lib.ts"],
        headRefName: "feature/foo",
        title: "Add release automation",
        labels: [],
      }),
    ).toEqual({
      kind: "warning",
      findings: [
        {
          severity: "warning",
          message: "missing ## Release Notes section",
        },
      ],
      isReleaseCandidate: false,
    });
  });

  test("blocks malformed release notes", () => {
    expect(
      analyzePullRequestForRelease(config, {
        body: [
          "## Release Notes",
          "",
          "- Performance: Faster.",
        ].join("\n"),
        changedPaths: ["src/lib.ts"],
        headRefName: "feature/foo",
        title: "Add release automation",
        labels: [],
      }),
    ).toEqual({
      kind: "failure",
      findings: [
        {
          severity: "failure",
          message: "unknown release note category: Performance",
        },
      ],
      isReleaseCandidate: false,
    });
  });

  test("warns when changelog is edited outside a release PR", () => {
    expect(
      analyzePullRequestForRelease(config, {
        body: [
          "## Release Notes",
          "",
          "- Added: Support release automation.",
        ].join("\n"),
        changedPaths: ["CHANGELOG.md"],
        headRefName: "feature/foo",
        title: "Add release automation",
        labels: [],
      }),
    ).toEqual({
      kind: "warning",
      findings: [
        {
          severity: "warning",
          message: "CHANGELOG.md can only be updated by release PRs",
        },
      ],
      isReleaseCandidate: false,
    });
  });

  test("treats release-branch PRs as candidates", () => {
    expect(
      analyzePullRequestForRelease(config, {
        body: [
          "## Release Notes",
          "",
          "- Added: Support release automation.",
        ].join("\n"),
        changedPaths: ["CHANGELOG.md"],
        headRefName: "release/v1.2.3",
        title: "release: v1.2.3",
        labels: ["release-pr"],
      }),
    ).toEqual({
      kind: "pass",
      findings: [],
      isReleaseCandidate: true,
    });
  });

  test("fails when a release candidate is missing release notes", () => {
    expect(
      analyzePullRequestForRelease(config, {
        body: "## Summary\n\nFeature work.",
        changedPaths: ["CHANGELOG.md"],
        headRefName: "release/v1.2.3",
        title: "release: v1.2.3",
        labels: ["release-pr"],
      }),
    ).toEqual({
      kind: "failure",
      findings: [
        {
          severity: "failure",
          message: "missing ## Release Notes section",
        },
      ],
      isReleaseCandidate: true,
    });
  });

  test("loads the pull request from github context", async () => {
    const github = {
      rest: {
        pulls: {
          get: () =>
            Promise.resolve({
              data: {
                body: [
                  "## Release Notes",
                  "",
                  "- Added: Support release automation.",
                ].join("\n"),
                title: "release: v1.2.3",
                head: { ref: "release/v1.2.3" },
                labels: [{ name: "release-pr" }],
              },
            }),
          listFiles: () => Promise.resolve({ data: [{ filename: "CHANGELOG.md" }] }),
        },
      },
    } as any;
    const context = {
      repo: { owner: "owner", repo: "repo" },
      payload: { pull_request: { number: 42 } },
    } as any;

    await expect(analyzePullRequestForReleaseFromGitHub(config, github, context)).resolves.toEqual({
      kind: "pass",
      findings: [],
      isReleaseCandidate: true,
    });
  });
});
