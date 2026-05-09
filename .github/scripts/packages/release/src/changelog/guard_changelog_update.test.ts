import { describe, expect, test } from "bun:test";
import { guardChangelogUpdate, hasChangelogPathChange } from "./guard_changelog_update.ts";

describe("hasChangelogPathChange", () => {
  test("detects the configured changelog path", () => {
    expect(hasChangelogPathChange(["src/lib.ts", "CHANGELOG.md"], "CHANGELOG.md")).toBe(true);
  });

  test("returns false when the changelog path is absent", () => {
    expect(hasChangelogPathChange(["src/lib.ts"], "CHANGELOG.md")).toBe(false);
  });
});

describe("guardChangelogUpdate", () => {
  test("allows non-changelog changes", () => {
    expect(
      guardChangelogUpdate({
        changedPaths: ["src/lib.ts"],
        changelogPath: "CHANGELOG.md",
        isReleasePr: false,
      }),
    ).toEqual({ kind: "allowed" });
  });

  test("blocks direct changelog edits outside release PRs", () => {
    expect(
      guardChangelogUpdate({
        changedPaths: ["CHANGELOG.md"],
        changelogPath: "CHANGELOG.md",
        isReleasePr: false,
      }),
    ).toEqual({
      kind: "blocked",
      reason: "CHANGELOG.md can only be updated by release PRs",
    });
  });

  test("allows changelog edits in release PRs", () => {
    expect(
      guardChangelogUpdate({
        changedPaths: ["CHANGELOG.md"],
        changelogPath: "CHANGELOG.md",
        isReleasePr: true,
      }),
    ).toEqual({ kind: "allowed" });
  });
});
