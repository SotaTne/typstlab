import { describe, expect, mock, test } from "bun:test";
import { findPullRequestsForCommit, toReleasePullRequestLike } from "./find_pr_for_commit.ts";

describe("toReleasePullRequestLike", () => {
  test("converts a merged GitHub PR into release PR shape", () => {
    expect(
      toReleasePullRequestLike({
        number: 12,
        body: null,
        html_url: "https://github.com/owner/repo/pull/12",
        title: "Add release automation",
        head: { ref: "feature/release" },
        labels: [{ name: "release-note" }, { name: null }],
        merged_at: "2026-05-16T00:00:00Z",
      }),
    ).toEqual({
      number: 12,
      body: "",
      htmlUrl: "https://github.com/owner/repo/pull/12",
      title: "Add release automation",
      headRefName: "feature/release",
      labels: ["release-note"],
    });
  });

  test("drops unmerged PRs", () => {
    expect(
      toReleasePullRequestLike({
        number: 12,
        body: "body",
        html_url: "https://github.com/owner/repo/pull/12",
        title: "Open PR",
        head: { ref: "feature/open" },
        labels: [],
        merged_at: null,
      }),
    ).toBeNull();
  });
});

describe("findPullRequestsForCommit", () => {
  test("loads merged associated PRs for a commit", async () => {
    const listPullRequestsAssociatedWithCommit = mock(() =>
      Promise.resolve({
        data: [
          {
            number: 12,
            body: "body",
            html_url: "https://github.com/owner/repo/pull/12",
            title: "Add feature",
            head: { ref: "feature/foo" },
            labels: [],
            merged_at: "2026-05-16T00:00:00Z",
          },
          {
            number: 13,
            body: "body",
            html_url: "https://github.com/owner/repo/pull/13",
            title: "Open feature",
            head: { ref: "feature/open" },
            labels: [],
            merged_at: null,
          },
        ],
      }),
    );
    const github = {
      rest: {
        repos: { listPullRequestsAssociatedWithCommit },
      },
    } as any;

    await expect(
      findPullRequestsForCommit(github, "owner", "repo", {
        sha: "abc123",
        subject: "Add feature",
      }),
    ).resolves.toEqual({
      commit: { sha: "abc123", subject: "Add feature" },
      pullRequests: [
        {
          number: 12,
          body: "body",
          htmlUrl: "https://github.com/owner/repo/pull/12",
          title: "Add feature",
          headRefName: "feature/foo",
          labels: [],
        },
      ],
    });
  });
});
