import { describe, expect, mock, test } from "bun:test";
import {
  collectPullRequestsForCommits,
  loadPullRequestLikeFromGitHub,
  toPullRequestLike,
} from "./collect_pull_requests.ts";

describe("toPullRequestLike", () => {
  test("converts GitHub payload into the internal PR shape", () => {
    expect(
      toPullRequestLike(
        {
          body: null,
          title: "Add release automation",
          head: { ref: "feature/foo" },
          labels: [{ name: "release-pr" }, { name: null }],
        },
        ["src/lib.ts", "CHANGELOG.md"],
      ),
    ).toEqual({
      body: "",
      changedPaths: ["src/lib.ts", "CHANGELOG.md"],
      headRefName: "feature/foo",
      title: "Add release automation",
      labels: ["release-pr"],
    });
  });
});

describe("loadPullRequestLikeFromGitHub", () => {
  test("loads a PR and its changed files from github", async () => {
    const get = mock(() =>
      Promise.resolve({
        data: {
          body: "## Release Notes\n\n- Added: Support release automation.",
          title: "Add release automation",
          head: { ref: "release/v1.2.3" },
          labels: [{ name: "release-pr" }],
        },
      }),
    );
    const listFiles = mock(() => Promise.resolve({ data: [{ filename: "CHANGELOG.md" }] }));
    const github = {
      rest: {
        pulls: {
          get,
          listFiles,
        },
      },
    } as any;
    const context = {
      repo: { owner: "owner", repo: "repo" },
      payload: {
        pull_request: { number: 42 },
      },
    } as any;

    await expect(loadPullRequestLikeFromGitHub(github, context)).resolves.toEqual({
      body: "## Release Notes\n\n- Added: Support release automation.",
      changedPaths: ["CHANGELOG.md"],
      headRefName: "release/v1.2.3",
      title: "Add release automation",
      labels: ["release-pr"],
    });
  });
});

describe("collectPullRequestsForCommits", () => {
  test("dedupes PRs and tracks commits without associated merged PRs", async () => {
    const listPullRequestsAssociatedWithCommit = mock((params: { commit_sha: string }) => {
      if (params.commit_sha === "aaa111") {
        return Promise.resolve({
          data: [
            {
              number: 10,
              body: "## Release Notes\n\n- Added: First.",
              html_url: "https://github.com/owner/repo/pull/10",
              title: "First PR",
              head: { ref: "feature/first" },
              labels: [],
              merged_at: "2026-05-16T00:00:00Z",
            },
          ],
        });
      }

      if (params.commit_sha === "bbb222") {
        return Promise.resolve({
          data: [
            {
              number: 10,
              body: "## Release Notes\n\n- Added: First.",
              html_url: "https://github.com/owner/repo/pull/10",
              title: "First PR",
              head: { ref: "feature/first" },
              labels: [],
              merged_at: "2026-05-16T00:00:00Z",
            },
            {
              number: 11,
              body: "## Release Notes\n\n- Fixed: Second.",
              html_url: "https://github.com/owner/repo/pull/11",
              title: "Second PR",
              head: { ref: "feature/second" },
              labels: [],
              merged_at: "2026-05-16T00:00:00Z",
            },
          ],
        });
      }

      return Promise.resolve({ data: [] });
    });
    const github = {
      rest: {
        repos: { listPullRequestsAssociatedWithCommit },
      },
    } as any;

    await expect(
      collectPullRequestsForCommits(github, "owner", "repo", [
        { sha: "aaa111", subject: "first" },
        { sha: "bbb222", subject: "second" },
        { sha: "ccc333", subject: "no pr" },
      ]),
    ).resolves.toEqual({
      pullRequests: [
        {
          number: 10,
          body: "## Release Notes\n\n- Added: First.",
          htmlUrl: "https://github.com/owner/repo/pull/10",
          title: "First PR",
          headRefName: "feature/first",
          labels: [],
        },
        {
          number: 11,
          body: "## Release Notes\n\n- Fixed: Second.",
          htmlUrl: "https://github.com/owner/repo/pull/11",
          title: "Second PR",
          headRefName: "feature/second",
          labels: [],
        },
      ],
      unassociatedCommits: [{ sha: "ccc333", subject: "no pr" }],
    });
  });
});
