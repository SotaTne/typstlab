import { isVersionTag } from "../version/version_validator.ts";
import type { GitRunner } from "./git.ts";

export async function resolvePreviousTag(
  runner: GitRunner,
  targetSha: string,
): Promise<string | null> {
  const result = await runner.exec([
    "describe",
    "--tags",
    "--abbrev=0",
    "--match",
    "v[0-9]*",
    targetSha,
  ]);

  if (result.exitCode !== 0) {
    return null;
  }

  const tag = result.stdout.trim();
  return isVersionTag(tag) ? tag : null;
}
