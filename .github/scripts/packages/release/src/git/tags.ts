import { isVersionTag } from "../version/version_validator.ts";
import type { GitRunner } from "./git.ts";
import { runGit } from "./git.ts";

export function parseTagLines(stdout: string): readonly string[] {
  return stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export function filterVersionTags(tags: readonly string[]): readonly string[] {
  return tags.filter(isVersionTag);
}

export async function listMergedVersionTags(
  runner: GitRunner,
  targetSha: string,
): Promise<readonly string[]> {
  const stdout = await runGit(runner, ["tag", "--merged", targetSha, "--list", "v*"]);
  return filterVersionTags(parseTagLines(stdout));
}

export async function getTagTargetSha(runner: GitRunner, tag: string): Promise<string | null> {
  const result = await runner.exec(["rev-list", "-n", "1", tag]);
  if (result.exitCode !== 0) {
    return null;
  }

  const sha = result.stdout.trim();
  return sha.length > 0 ? sha : null;
}
