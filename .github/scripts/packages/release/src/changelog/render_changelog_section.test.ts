import { describe, expect, test } from "bun:test";
import type { ChangelogDocument } from "./struct.ts";
import { renderChangelogDocument, renderChangelogSection, renderReleaseNoteEntries } from "./render_changelog_section.ts";

const repository = {
  owner: "SotaTne",
  repo: "typstlab",
};

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

describe("renderChangelogSection", () => {
  test("renders a release section with grouped entries", () => {
    expect(
      renderChangelogSection(
        {
          version: "0.2.0",
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
            {
              category: "Fixed",
              text: "Normalize changelog rendering.",
              pullRequest: null,
            },
          ],
        },
        repository,
      ),
    ).toBe(
      [
        "## [0.2.0] - 2026-05-09",
        "",
        "### Added",
        "",
        "- Support release automation. ([#12](https://github.com/SotaTne/typstlab/pull/12))",
        "",
        "### Fixed",
        "",
        "- Normalize changelog rendering.",
        "",
        "### Other",
        "",
        "- Keep the release flow deterministic.",
      ].join("\n"),
    );
  });
});

describe("renderChangelogDocument", () => {
  test("renders a document with links and grouped entries", () => {
    const document: ChangelogDocument = {
      title: "Changelog",
      header: "All notable changes.",
      releases: [
        {
          version: "0.2.0",
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
    };

    expect(renderChangelogDocument(document, repository)).toBe(
      [
        "# Changelog",
        "",
        "All notable changes.",
        "",
        "## [0.2.0] - 2026-05-09",
        "",
        "### Added",
        "",
        "- Support release automation. ([#12](https://github.com/SotaTne/typstlab/pull/12))",
        "",
        "### Other",
        "",
        "- Keep the release flow deterministic.",
      ].join("\n"),
    );
  });
});
