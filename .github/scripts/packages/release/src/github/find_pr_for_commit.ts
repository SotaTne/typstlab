import type { GitCommit } from "../git/git.ts";
import type { GitHubAssociatedPullRequest, GitHubClientLike, ReleasePullRequestLike } from "./struct.ts";
import { getPullRequestsAssociatedWithCommit } from "./github_client.ts";

export type AssociatedPullRequestsForCommit = {
  commit: GitCommit;
  pullRequests: readonly ReleasePullRequestLike[];
};

export function toReleasePullRequestLike(
  pullRequest: GitHubAssociatedPullRequest,
): ReleasePullRequestLike | null {
  if (pullRequest.merged_at === null) {
    return null;
  }

  return {
    number: pullRequest.number,
    body: pullRequest.body ?? "",
    htmlUrl: pullRequest.html_url,
    headRefName: pullRequest.head.ref,
    title: pullRequest.title,
    labels: pullRequest.labels.flatMap((label) => (label.name ? [label.name] : [])),
  };
}

export async function findPullRequestsForCommit(
  github: GitHubClientLike,
  owner: string,
  repo: string,
  commit: GitCommit,
): Promise<AssociatedPullRequestsForCommit> {
  const pullRequests = await getPullRequestsAssociatedWithCommit(github, owner, repo, commit.sha);

  return {
    commit,
    pullRequests: pullRequests.flatMap((pullRequest) => {
      const converted = toReleasePullRequestLike(pullRequest);
      return converted ? [converted] : [];
    }),
  };
}
