import { describe, expect, test } from "bun:test";
import { extractReleaseNotesSection } from "./extract_release_notes.ts";

describe("extractReleaseNotesSection", () => {
  test("extracts content until the next level-2 heading", () => {
    const result = extractReleaseNotesSection(
      [
        "## Summary",
        "",
        "Internal summary.",
        "",
        "## Release Notes",
        "",
        "- Added: Support release PR generation.",
        "",
        "## Test Plan",
        "",
        "- bun test",
      ].join("\n"),
      "Release Notes",
    );

    expect(result).toEqual({
      kind: "found",
      content: "- Added: Support release PR generation.",
    });
  });

  test("returns missing when the heading is absent", () => {
    expect(extractReleaseNotesSection("## Summary\n\nNo notes.", "Release Notes")).toEqual({
      kind: "missing",
    });
  });
});
