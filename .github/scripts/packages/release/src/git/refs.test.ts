import { describe, expect, test } from "bun:test";
import type { GitRunner } from "./git.ts";
import { getCurrentHeadSha, resolveRefSha } from "./refs.ts";

describe("resolveRefSha", () => {
  test("resolves a ref to a sha", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["rev-parse", "--verify", "origin/main"]);
        return { exitCode: 0, stdout: "abc123\n", stderr: "" };
      },
    };

    await expect(resolveRefSha(runner, "origin/main")).resolves.toBe("abc123");
  });
});

describe("getCurrentHeadSha", () => {
  test("resolves HEAD", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["rev-parse", "--verify", "HEAD"]);
        return { exitCode: 0, stdout: "head123\n", stderr: "" };
      },
    };

    await expect(getCurrentHeadSha(runner)).resolves.toBe("head123");
  });
});
