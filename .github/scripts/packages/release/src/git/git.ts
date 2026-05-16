import type { AsyncFunctionArguments } from "@actions/github-script";

export type GitCommandResult = {
  exitCode: number;
  stdout: string;
  stderr: string;
};

export type GitRunner = {
  exec(args: readonly string[]): Promise<GitCommandResult>;
};

export type GitCommit = {
  sha: string;
  subject: string;
};

export type GitActionExec = AsyncFunctionArguments["exec"];

export function createGitRunner(exec: GitActionExec, cwd?: string): GitRunner {
  return {
    async exec(args) {
      const result = await exec.getExecOutput("git", [...args], {
        cwd,
        ignoreReturnCode: true,
        silent: true,
      });

      return {
        exitCode: result.exitCode,
        stdout: result.stdout,
        stderr: result.stderr,
      };
    },
  };
}

export async function runGit(runner: GitRunner, args: readonly string[]): Promise<string> {
  const result = await runner.exec(args);
  if (!isSuccessExitCode(result.exitCode)) {
    throw new Error(formatGitError(args, result));
  }

  return result.stdout.trim();
}

export function isSuccessExitCode(exitCode: number): boolean {
  return exitCode === 0;
}

export function formatGitError(args: readonly string[], result: GitCommandResult): string {
  const stderr = result.stderr.trim();
  const stdout = result.stdout.trim();
  const detail = stderr.length > 0 ? stderr : stdout;
  const suffix = detail.length > 0 ? `: ${detail}` : "";
  return `git ${args.join(" ")} failed with exit code ${result.exitCode}${suffix}`;
}
