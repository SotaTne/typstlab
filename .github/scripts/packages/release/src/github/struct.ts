import type { AsyncFunctionArguments } from "@actions/github-script";
import type { RepositoryRef } from "../changelog/struct.ts";

export type GitHubPullRequestLabel = {
  name: string | null | undefined;
};

export type GitHubPullRequestHead = {
  ref: string;
};

export type GitHubPullRequest = {
  body: string | null;
  title: string;
  head: GitHubPullRequestHead;
  labels: readonly GitHubPullRequestLabel[];
};

export type GitHubPullRequestFile = {
  filename: string;
};

export type GitHubActionArgs = Pick<AsyncFunctionArguments, "github" | "context" | "core">;

export type GitHubClientLike = GitHubActionArgs["github"];
export type GitHubContextLike = GitHubActionArgs["context"];
export type GitHubCoreLike = GitHubActionArgs["core"];

export type GitHubRestPullsApi = {
  get(params: {
    owner: string;
    repo: string;
    pull_number: number;
  }): Promise<{
    data: GitHubPullRequest;
  }>;
  listFiles(params: {
    owner: string;
    repo: string;
    pull_number: number;
    per_page?: number;
  }): Promise<{
    data: readonly GitHubPullRequestFile[];
  }>;
};

export type PullRequestLike = {
  body: string;
  changedPaths: readonly string[];
  headRefName: string;
  title: string;
  labels: readonly string[];
};
