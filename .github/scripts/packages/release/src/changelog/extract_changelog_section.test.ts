import { describe, expect, test } from "bun:test";
import { extractChangelogReleaseSection, parseChangelogDocument } from "./extract_changelog_section.ts";

describe("parseChangelogDocument", () => {
  test("parses a standard changelog document", () => {
    expect(
      parseChangelogDocument(
        [
          "# Changelog",
          "",
          "All notable changes to this project will be documented in this file.",
          "",
          "## [0.1.0] - 2026-05-09",
          "",
          "### Added",
          "",
          "- Support release automation. ([#12](https://github.com/SotaTne/typstlab/pull/12))",
          "",
          "### Other",
          "",
          "- Keep the release flow deterministic.",
        ].join("\n"),
      ),
    ).toEqual({
      kind: "parsed",
      document: {
        title: "Changelog",
        header: [
          "All notable changes to this project will be documented in this file.",
        ].join("\n"),
        releases: [
          {
            version: "0.1.0",
            date: "2026-05-09",
            entries: [
              {
                category: "Added",
                text: "Support release automation.",
                pullRequest: { id: 12 },
              },
              {
                category: "Other",
                text: "Keep the release flow deterministic.",
                pullRequest: null,
              },
            ],
          },
        ],
      },
    });
  });

  test("rejects malformed documents", () => {
    expect(parseChangelogDocument("Changelog\n\nNo heading.")).toEqual({
      kind: "invalid",
      error: "changelog title must start with '# '",
    });
  });
});

describe("extractChangelogReleaseSection", () => {
  test("extracts the requested release section", () => {
    expect(
      extractChangelogReleaseSection(
        [
          "# Changelog",
          "",
          "All notable changes.",
          "",
          "## [0.1.0] - 2026-05-01",
          "",
          "### Other",
          "",
          "- Initial release.",
          "",
          "## [0.2.0] - 2026-05-09",
          "",
          "### Added",
          "",
          "- Support release automation.",
        ].join("\n"),
        "0.2.0",
      ),
    ).toEqual({
      kind: "found",
      content: [
        "## [0.2.0] - 2026-05-09",
        "",
        "### Added",
        "",
        "- Support release automation.",
      ].join("\n"),
    });
  });
});
