import { describe, expect, test } from "bun:test";
import type { GitRunner } from "./git.ts";
import { resolvePreviousTag } from "./resolve_previous_tag.ts";

describe("resolvePreviousTag", () => {
  test("resolves the nearest stable version tag reachable from target sha", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual([
          "describe",
          "--tags",
          "--abbrev=0",
          "--match",
          "v[0-9]*",
          "abc123",
        ]);
        return { exitCode: 0, stdout: "v1.2.3\n", stderr: "" };
      },
    };

    await expect(resolvePreviousTag(runner, "abc123")).resolves.toBe("v1.2.3");
  });

  test("returns null when no previous tag exists", async () => {
    const runner: GitRunner = {
      async exec() {
        return { exitCode: 128, stdout: "", stderr: "No names found" };
      },
    };

    await expect(resolvePreviousTag(runner, "abc123")).resolves.toBeNull();
  });

  test("ignores non-stable tags matched by git describe", async () => {
    const runner: GitRunner = {
      async exec() {
        return { exitCode: 0, stdout: "v1.2.3-pre\n", stderr: "" };
      },
    };

    await expect(resolvePreviousTag(runner, "abc123")).resolves.toBeNull();
  });
});
