import type { GitHubClientLike, GitHubPullRequest, GitHubPullRequestFile } from "./struct.ts";

export async function getPullRequest(
  github: GitHubClientLike,
  owner: string,
  repo: string,
  pullNumber: number,
): Promise<GitHubPullRequest> {
  const { data } = await github.rest.pulls.get({
    owner,
    repo,
    pull_number: pullNumber,
  });

  return data;
}

export async function getPullRequestFiles(
  github: GitHubClientLike,
  owner: string,
  repo: string,
  pullNumber: number,
): Promise<readonly GitHubPullRequestFile[]> {
  if (github.paginate) {
    return github.paginate(
      github.rest.pulls.listFiles,
      {
        owner,
        repo,
        pull_number: pullNumber,
      },
    );
  }

  const { data } = await github.rest.pulls.listFiles({
    owner,
    repo,
    pull_number: pullNumber,
  });

  return data;
}
