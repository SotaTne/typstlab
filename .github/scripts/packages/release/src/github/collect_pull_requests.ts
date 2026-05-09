import type { AsyncFunctionArguments } from "@actions/github-script";
import type { GitHubClientLike, GitHubContextLike, PullRequestLike } from "./struct.ts";
import { getPullRequest } from "./github_client.ts";
import { getPullRequestChangedPaths } from "./get_pr_changed_files.ts";

export function toPullRequestLike(
  pullRequest: {
    body: string | null;
    title: string;
    head: {
      ref: string;
    };
    labels: readonly { name: string | null | undefined }[];
  },
  changedPaths: readonly string[],
): PullRequestLike {
  return {
    body: pullRequest.body ?? "",
    changedPaths: [...changedPaths],
    headRefName: pullRequest.head.ref,
    title: pullRequest.title,
    labels: pullRequest.labels.flatMap((label) => (label.name ? [label.name] : [])),
  };
}

export async function loadPullRequestLikeFromGitHub(
  github: GitHubClientLike,
  context: GitHubContextLike,
  pullNumber = context.payload.pull_request?.number,
): Promise<PullRequestLike> {
  if (pullNumber === undefined) {
    throw new Error("pull request number is required");
  }

  const pullRequest = await getPullRequest(github, context.repo.owner, context.repo.repo, pullNumber);
  const changedPaths = await getPullRequestChangedPaths(
    github,
    context.repo.owner,
    context.repo.repo,
    pullNumber,
  );

  return toPullRequestLike(pullRequest, changedPaths);
}

export async function loadPullRequestLikeFromActionArgs(
  args: Pick<AsyncFunctionArguments, "github" | "context">,
  pullNumber = args.context.payload.pull_request?.number,
): Promise<PullRequestLike> {
  return loadPullRequestLikeFromGitHub(args.github, args.context, pullNumber);
}
