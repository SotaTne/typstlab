import { describe, expect, test } from "bun:test";
import type { GitRunner } from "./git.ts";
import { filterVersionTags, getTagTargetSha, listMergedVersionTags, parseTagLines } from "./tags.ts";

describe("parseTagLines", () => {
  test("parses non-empty tag lines", () => {
    expect(parseTagLines("v0.1.0\n\n v0.2.0 \n")).toEqual(["v0.1.0", "v0.2.0"]);
  });
});

describe("filterVersionTags", () => {
  test("keeps only stable vX.Y.Z tags", () => {
    expect(filterVersionTags(["v1.2.3", "1.2.3", "v1.2", "v1.2.3-pre", "foo"])).toEqual([
      "v1.2.3",
    ]);
  });
});

describe("listMergedVersionTags", () => {
  test("lists merged version tags for a target sha", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["tag", "--merged", "HEAD", "--list", "v*"]);
        return { exitCode: 0, stdout: "v1.0.0\nv1.1.0-pre\nv1.1.0\n", stderr: "" };
      },
    };

    await expect(listMergedVersionTags(runner, "HEAD")).resolves.toEqual(["v1.0.0", "v1.1.0"]);
  });
});

describe("getTagTargetSha", () => {
  test("returns the sha a tag points to", async () => {
    const runner: GitRunner = {
      async exec(args) {
        expect(args).toEqual(["rev-list", "-n", "1", "v1.0.0"]);
        return { exitCode: 0, stdout: "abc123\n", stderr: "" };
      },
    };

    await expect(getTagTargetSha(runner, "v1.0.0")).resolves.toBe("abc123");
  });

  test("returns null when the tag does not exist", async () => {
    const runner: GitRunner = {
      async exec() {
        return { exitCode: 128, stdout: "", stderr: "unknown revision" };
      },
    };

    await expect(getTagTargetSha(runner, "v1.0.0")).resolves.toBeNull();
  });
});
