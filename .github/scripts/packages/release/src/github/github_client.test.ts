import { describe, expect, test, mock } from "bun:test";
import {
  getPullRequest,
  getPullRequestFiles,
  getPullRequestsAssociatedWithCommit,
} from "./github_client.ts";

describe("github_client", () => {
  test("getPullRequest returns the underlying PR data", async () => {
    const get = mock(() =>
      Promise.resolve({
        data: {
          body: "body",
          title: "title",
          head: { ref: "feature/foo" },
          labels: [{ name: "release-pr" }],
        },
      }),
    );

    const github = {
      rest: {
        pulls: {
          get,
          listFiles: mock(() => Promise.resolve({ data: [] })),
        },
      },
    } as any;

    await expect(getPullRequest(github, "owner", "repo", 12)).resolves.toEqual({
      body: "body",
      title: "title",
      head: { ref: "feature/foo" },
      labels: [{ name: "release-pr" }],
    });
    expect(get).toHaveBeenCalledWith({
      owner: "owner",
      repo: "repo",
      pull_number: 12,
    });
  });

  test("getPullRequestFiles returns the listed files when paginate is absent", async () => {
    const listFiles = mock(() => Promise.resolve({ data: [{ filename: "CHANGELOG.md" }] }));
    const github = {
      rest: {
        pulls: {
          get: mock(() => Promise.resolve({ data: null })),
          listFiles,
        },
      },
    } as any;

    await expect(getPullRequestFiles(github, "owner", "repo", 12)).resolves.toEqual([
      { filename: "CHANGELOG.md" },
    ]);
  });

  test("getPullRequestsAssociatedWithCommit returns associated PRs", async () => {
    const listPullRequestsAssociatedWithCommit = mock(() =>
      Promise.resolve({
        data: [
          {
            number: 123,
            body: "body",
            html_url: "https://github.com/owner/repo/pull/123",
            title: "title",
            head: { ref: "feature/foo" },
            labels: [],
            merged_at: "2026-05-16T00:00:00Z",
          },
        ],
      }),
    );
    const github = {
      rest: {
        pulls: {
          get: mock(() => Promise.resolve({ data: null })),
          listFiles: mock(() => Promise.resolve({ data: [] })),
        },
        repos: {
          listPullRequestsAssociatedWithCommit,
        },
      },
    } as any;

    await expect(
      getPullRequestsAssociatedWithCommit(github, "owner", "repo", "abc123"),
    ).resolves.toEqual([
      {
        number: 123,
        body: "body",
        html_url: "https://github.com/owner/repo/pull/123",
        title: "title",
        head: { ref: "feature/foo" },
        labels: [],
        merged_at: "2026-05-16T00:00:00Z",
      },
    ]);
    expect(listPullRequestsAssociatedWithCommit).toHaveBeenCalledWith({
      owner: "owner",
      repo: "repo",
      commit_sha: "abc123",
    });
  });
});
