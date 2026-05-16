import type { GitCommit, GitRunner } from "./git.ts";
import { runGit } from "./git.ts";

export function parseGitLog(stdout: string): readonly GitCommit[] {
  return stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => {
      const separatorIndex = line.indexOf("\t");
      if (separatorIndex === -1) {
        return { sha: line, subject: "" };
      }

      return {
        sha: line.slice(0, separatorIndex),
        subject: line.slice(separatorIndex + 1),
      };
    });
}

export async function listCommitsBetween(
  runner: GitRunner,
  previousTag: string | null,
  targetSha: string,
): Promise<readonly GitCommit[]> {
  const range = previousTag === null ? targetSha : `${previousTag}..${targetSha}`;
  const stdout = await runGit(runner, ["log", "--format=%H%x09%s", range]);
  return parseGitLog(stdout);
}
