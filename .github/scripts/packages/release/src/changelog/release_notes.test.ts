import { describe, expect, test } from "bun:test";
import { extractReleaseNotesSection, validateReleaseNotes } from "./parser";
import { DEFAULT_RELEASE_NOTE_CATEGORIES } from "./struct";
import { renderReleaseNoteEntries } from "./writer";

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

describe("validateReleaseNotes", () => {
  test("accepts Keep a Changelog categories", () => {
    const result = validateReleaseNotes("- Added: Support mise installation.", {
      allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
    });

    expect(result).toEqual({
      kind: "entries",
      entries: [{ category: "Added", text: "Support mise installation." }],
    });
  });

  test("uses Other for entries without a category prefix", () => {
    const result = validateReleaseNotes("- Improve release automation diagnostics.", {
      allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
    });

    expect(result).toEqual({
      kind: "entries",
      entries: [{ category: "Other", text: "Improve release automation diagnostics." }],
    });
  });

  test("accepts multiple Other entries", () => {
    const result = validateReleaseNotes(
      [
        "- Improve release automation diagnostics.",
        "- Document release PR metadata.",
        "- Keep changelog generation deterministic.",
      ].join("\n"),
      {
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
      },
    );

    expect(result).toEqual({
      kind: "entries",
      entries: [
        { category: "Other", text: "Improve release automation diagnostics." },
        { category: "Other", text: "Document release PR metadata." },
        { category: "Other", text: "Keep changelog generation deterministic." },
      ],
    });
  });

  test("accepts mixed categorized and Other entries in order", () => {
    const result = validateReleaseNotes(
      [
        "- Added: Support release PR generation.",
        "- Improve release automation diagnostics.",
        "- Fixed: Reject malformed release note entries.",
        "- Document release PR metadata.",
      ].join("\n"),
      {
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
      },
    );

    expect(result).toEqual({
      kind: "entries",
      entries: [
        { category: "Added", text: "Support release PR generation." },
        { category: "Other", text: "Improve release automation diagnostics." },
        { category: "Fixed", text: "Reject malformed release note entries." },
        { category: "Other", text: "Document release PR metadata." },
      ],
    });
  });

  test("treats a single N/A item as ignored", () => {
    expect(
      validateReleaseNotes("- N/A", {
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
      }),
    ).toEqual({ kind: "ignored" });
  });

  test("rejects N/A mixed with changelog entries", () => {
    expect(
      validateReleaseNotes("- N/A\n- Fixed: Do not mix entries.", {
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
      }),
    ).toEqual({
      kind: "invalid",
      errors: ["- N/A must be the only release note item"],
    });
  });

  test("rejects unknown categories", () => {
    expect(
      validateReleaseNotes("- Performance: Faster release generation.", {
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
      }),
    ).toEqual({
      kind: "invalid",
      errors: ["unknown release note category: Performance"],
    });
  });

  test("rejects non-list content and malformed entries", () => {
    expect(
      validateReleaseNotes("Added: Missing list marker.\n- Fixed:", {
        allowedCategories: DEFAULT_RELEASE_NOTE_CATEGORIES,
      }),
    ).toEqual({
      kind: "invalid",
      errors: [
        "release note line must start with '- ': Added: Missing list marker.",
        "release note text must not be empty: - Fixed:",
      ],
    });
  });
});

describe("renderReleaseNoteEntries", () => {
  test("renders categorized and Other entries", () => {
    expect(
      renderReleaseNoteEntries([
        { category: "Added", text: "Support release PR generation." },
        { category: "Other", text: "Improve release automation diagnostics." },
        { category: "Fixed", text: "Reject malformed release note entries." },
      ]),
    ).toBe(
      [
        "- Added: Support release PR generation.",
        "- Improve release automation diagnostics.",
        "- Fixed: Reject malformed release note entries.",
      ].join("\n"),
    );
  });
});
