import { describe, expect, test, mock } from "bun:test";
import { getPullRequest, getPullRequestFiles } from "./github_client.ts";

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
});
