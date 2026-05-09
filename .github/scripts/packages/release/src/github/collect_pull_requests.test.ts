import { describe, expect, mock, test } from "bun:test";
import { loadPullRequestLikeFromGitHub, toPullRequestLike } from "./collect_pull_requests.ts";

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
