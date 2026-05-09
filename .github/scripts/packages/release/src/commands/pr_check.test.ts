import { describe, expect, test } from "bun:test";
import {
  analyzePullRequestForRelease,
  analyzePullRequestForReleaseFromGitHub,
  isReleasePrCandidate,
} from "./pr_check.ts";

const config = {
  changelogPath: "CHANGELOG.md",
  changelogTitle: "Changelog",
  changelogHeader: "Header",
  releaseNotesHeading: "Release Notes",
  allowedCategories: ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security", "Other"],
  releaseBranchPattern: "release/v{version}",
  releaseTagPattern: "v{version}",
  releasePrTitlePattern: "release: v{version}",
  releasePrLabel: "release-pr",
  releaseDraft: true,
};

describe("isReleasePrCandidate", () => {
  test("matches branch, title, and label", () => {
    expect(
      isReleasePrCandidate(config, {
        headRefName: "release/v1.2.3",
        title: "something else",
        labels: [],
      }),
    ).toBe(true);

    expect(
      isReleasePrCandidate(config, {
        headRefName: "feature/foo",
        title: "release: v1.2.3",
        labels: [],
      }),
    ).toBe(true);

    expect(
      isReleasePrCandidate(config, {
        headRefName: "feature/foo",
        title: "something else",
        labels: ["release-pr"],
      }),
    ).toBe(true);
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
