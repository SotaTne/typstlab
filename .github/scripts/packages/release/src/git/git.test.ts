import { describe, expect, test } from "bun:test";
import type { GitRunner } from "./git.ts";
import { formatGitError, isSuccessExitCode, runGit } from "./git.ts";

describe("runGit", () => {
  test("returns trimmed stdout when git succeeds", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["rev-parse", "HEAD"]);
        return { exitCode: 0, stdout: "abc123\n", stderr: "" };
      },
    };

    await expect(runGit(runner, ["rev-parse", "HEAD"])).resolves.toBe("abc123");
  });

  test("throws with command details when git fails", async () => {
    const runner: GitRunner = {
      async exec() {
        return { exitCode: 128, stdout: "", stderr: "bad revision" };
      },
    };

    await expect(runGit(runner, ["rev-parse", "missing"])).rejects.toThrow(
      "git rev-parse missing failed with exit code 128: bad revision",
    );
  });
});

describe("formatGitError", () => {
  test("falls back to stdout when stderr is empty", () => {
    expect(
      formatGitError(["describe"], {
        exitCode: 1,
        stdout: "no names found",
        stderr: "",
      }),
    ).toBe("git describe failed with exit code 1: no names found");
  });
});

describe("isSuccessExitCode", () => {
  test("treats only zero as success", () => {
    expect(isSuccessExitCode(0)).toBe(true);
    expect(isSuccessExitCode(1)).toBe(false);
    expect(isSuccessExitCode(128)).toBe(false);
  });
});
