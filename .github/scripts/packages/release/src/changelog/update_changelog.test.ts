import { describe, expect, test } from "bun:test";
import type { ChangelogDocument } from "./struct.ts";
import { upsertChangelogRelease } from "./update_changelog.ts";

describe("upsertChangelogRelease", () => {
  test("inserts the new release at the top", () => {
    const document: ChangelogDocument = {
      title: "Changelog",
      header: "All notable changes.",
      releases: [
        {
          version: "0.1.0",
          date: "2026-05-01",
          entries: [
            { category: "Other", text: "Initial release.", pullRequest: null },
          ],
        },
      ],
    };

    expect(
      upsertChangelogRelease(document, {
        version: "0.2.0",
        date: "2026-05-09",
        entries: [
          { category: "Added", text: "Support release automation.", pullRequest: { id: 12 } },
        ],
      }),
    ).toEqual({
      title: "Changelog",
      header: "All notable changes.",
      releases: [
        {
          version: "0.2.0",
          date: "2026-05-09",
          entries: [
            { category: "Added", text: "Support release automation.", pullRequest: { id: 12 } },
          ],
        },
        {
          version: "0.1.0",
          date: "2026-05-01",
          entries: [
            { category: "Other", text: "Initial release.", pullRequest: null },
          ],
        },
      ],
    });
  });

  test("replaces an existing release version", () => {
    const document: ChangelogDocument = {
      title: "Changelog",
      header: "All notable changes.",
      releases: [
        {
          version: "0.1.0",
          date: "2026-05-01",
          entries: [
            { category: "Other", text: "Initial release.", pullRequest: null },
          ],
        },
      ],
    };

    expect(
      upsertChangelogRelease(document, {
        version: "0.1.0",
        date: "2026-05-09",
        entries: [
          { category: "Fixed", text: "Normalize changelog rendering.", pullRequest: null },
        ],
      }),
    ).toEqual({
      title: "Changelog",
      header: "All notable changes.",
      releases: [
        {
          version: "0.1.0",
          date: "2026-05-09",
          entries: [
            { category: "Fixed", text: "Normalize changelog rendering.", pullRequest: null },
          ],
        },
      ],
    });
  });
});
