import type {
  ChangelogEntry,
  ChangelogRelease,
  ExtractChangelogSectionResult,
  ParseChangelogDocumentResult,
  PullRequestRef,
} from "./struct.ts";
import { renderChangelogSection } from "./render_changelog_section.ts";

export function extractChangelogReleaseSection(
  markdown: string,
  version: string,
): ExtractChangelogSectionResult {
  const parsed = parseChangelogDocument(markdown);

  if (parsed.kind === "invalid") {
    return parsed;
  }

  const release = parsed.document.releases.find((item) => item.version === version);

  if (release === undefined) {
    return { kind: "missing" };
  }

  return {
    kind: "found",
    content: renderChangelogSection(release),
  };
}

export function parseChangelogDocument(markdown: string): ParseChangelogDocumentResult {
  const lines = markdown.split(/\r?\n/);

  if (lines.length === 0 || lines[0] === undefined) {
    return { kind: "invalid", error: "changelog document is empty" };
  }

  const titleLine = lines[0].trim();

  if (!titleLine.startsWith("# ")) {
    return { kind: "invalid", error: "changelog title must start with '# '" };
  }

  const title = titleLine.slice(2).trim();
  const releaseStart = lines.findIndex((line, index) => index > 0 && isVersionHeading(line));
  const headerLines = releaseStart === -1 ? lines.slice(1) : lines.slice(1, releaseStart);
  const releases: ChangelogRelease[] = [];

  let index = releaseStart === -1 ? lines.length : releaseStart;

  while (index < lines.length) {
    while (index < lines.length && lines[index]?.trim().length === 0) {
      index += 1;
    }

    if (index >= lines.length) {
      break;
    }

    const releaseHeading = lines[index]?.trim();
    if (releaseHeading === undefined || !isVersionHeading(releaseHeading)) {
      return {
        kind: "invalid",
        error: `expected release heading at line ${index + 1}: ${releaseHeading ?? ""}`,
      };
    }

    const releaseMatch = releaseHeading.match(/^## \[(\d+\.\d+\.\d+)\] - (\d{4}-\d{2}-\d{2})$/);

    if (releaseMatch === null) {
      return {
        kind: "invalid",
        error: `invalid release heading: ${releaseHeading}`,
      };
    }

    const version = releaseMatch[1];
    const date = releaseMatch[2];

    if (version === undefined || date === undefined) {
      return {
        kind: "invalid",
        error: `invalid release heading: ${releaseHeading}`,
      };
    }

    index += 1;

    const sectionLines: string[] = [];
    while (index < lines.length && !isVersionHeading(lines[index] ?? "")) {
      sectionLines.push(lines[index] ?? "");
      index += 1;
    }

    const release = parseChangelogReleaseSection(sectionLines, version, date);
    if (release.kind === "invalid") {
      return release;
    }

    releases.push(release.release);
  }

  return {
    kind: "parsed",
    document: {
      title,
      header: trimBlankLines(headerLines).join("\n"),
      releases,
    },
  };
}

type ParseChangelogReleaseSectionResult =
  | {
      kind: "parsed";
      release: ChangelogRelease;
    }
  | {
      kind: "invalid";
      error: string;
    };

function parseChangelogReleaseSection(
  lines: readonly string[],
  version: string,
  date: string,
): ParseChangelogReleaseSectionResult {
  const entries: ChangelogEntry[] = [];
  let currentCategory: string | null = null;

  for (const line of trimBlankLines(lines)) {
    const trimmed = line.trim();

    if (trimmed.length === 0) {
      continue;
    }

    if (trimmed.startsWith("### ")) {
      currentCategory = trimmed.slice(4).trim();

      if (currentCategory.length === 0) {
        return {
          kind: "invalid",
          error: `empty category heading in release ${version}`,
        };
      }

      continue;
    }

    if (!trimmed.startsWith("- ")) {
      return {
        kind: "invalid",
        error: `release entry must start with '- ': ${trimmed}`,
      };
    }

    if (currentCategory === null) {
      return {
        kind: "invalid",
        error: `release entry appears before a category heading in ${version}: ${trimmed}`,
      };
    }

    entries.push(parseChangelogEntry(trimmed, currentCategory));
  }

  return {
    kind: "parsed",
    release: {
      version,
      date,
      entries,
    },
  };
}

function parseChangelogEntry(line: string, category: string): ChangelogEntry {
  const item = line.slice(2).trim();
  const linkMatch = item.match(/^(.*)\s+\(\[#(\d+)\]\(([^)]+)\)\)$/);

  if (linkMatch === null) {
    return {
      category: category as ChangelogEntry["category"],
      text: item,
      pullRequest: null,
    };
  }

  const text = linkMatch[1]?.trim();
  const pullRequestId = linkMatch[2];

  if (text === undefined || text.length === 0) {
    return {
      category: category as ChangelogEntry["category"],
      text: item,
      pullRequest: null,
    };
  }

  return {
    category: category as ChangelogEntry["category"],
    text,
    pullRequest: {
      id: Number(pullRequestId),
    } satisfies PullRequestRef,
  };
}

function isVersionHeading(line: string): boolean {
  return line.trim().startsWith("## [") && line.trim().includes("] - ");
}

function trimBlankLines(lines: readonly string[]): string[] {
  let start = 0;
  let end = lines.length;

  while (start < end && (lines[start] ?? "").trim().length === 0) {
    start += 1;
  }

  while (end > start && (lines[end - 1] ?? "").trim().length === 0) {
    end -= 1;
  }

  return lines.slice(start, end);
}
