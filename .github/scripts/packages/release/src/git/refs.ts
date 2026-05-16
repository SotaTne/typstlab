import type { GitRunner } from "./git.ts";
import { runGit } from "./git.ts";

export async function resolveRefSha(runner: GitRunner, ref: string): Promise<string> {
  return runGit(runner, ["rev-parse", "--verify", ref]);
}

export async function getCurrentHeadSha(runner: GitRunner): Promise<string> {
  return resolveRefSha(runner, "HEAD");
}
