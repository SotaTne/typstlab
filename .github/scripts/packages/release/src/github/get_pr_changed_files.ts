import type { GitHubClientLike } from "./struct.ts";
import { getPullRequestFiles } from "./github_client.ts";

export async function getPullRequestChangedPaths(
  github: GitHubClientLike,
  owner: string,
  repo: string,
  pullNumber: number,
): Promise<string[]> {
  const files = await getPullRequestFiles(github, owner, repo, pullNumber);
  return files.map((file) => file.filename);
}
