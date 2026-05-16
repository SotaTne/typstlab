import { describe, expect, test } from "bun:test";
import type { GitRunner } from "./git.ts";
import { listCommitsBetween, parseGitLog } from "./list_commits_between.ts";

describe("parseGitLog", () => {
  test("parses sha and subject pairs", () => {
    expect(parseGitLog("abc123\tAdd release notes\nfff999\tFix checksum\n")).toEqual([
      { sha: "abc123", subject: "Add release notes" },
      { sha: "fff999", subject: "Fix checksum" },
    ]);
  });

  test("keeps commits without subjects parseable", () => {
    expect(parseGitLog("abc123\n")).toEqual([{ sha: "abc123", subject: "" }]);
  });
});

describe("listCommitsBetween", () => {
  test("uses previousTag..targetSha when previous tag exists", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["log", "--format=%H%x09%s", "v1.0.0..HEAD"]);
        return { exitCode: 0, stdout: "abc123\tAdd feature\n", stderr: "" };
      },
    };

    await expect(listCommitsBetween(runner, "v1.0.0", "HEAD")).resolves.toEqual([
      { sha: "abc123", subject: "Add feature" },
    ]);
  });

  test("uses targetSha alone for initial release mode", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["log", "--format=%H%x09%s", "HEAD"]);
        return { exitCode: 0, stdout: "abc123\tInitial release\n", stderr: "" };
      },
    };

    await expect(listCommitsBetween(runner, null, "HEAD")).resolves.toEqual([
      { sha: "abc123", subject: "Initial release" },
    ]);
  });
});
