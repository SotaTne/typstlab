import type {
  ChangelogDocument,
  ChangelogEntry,
  ChangelogRelease,
  ReleaseNoteCategory,
  ReleaseNoteEntry,
  RepositoryRef,
} from "./struct.ts";
import { DEFAULT_RELEASE_NOTE_CATEGORIES } from "./struct.ts";

export function renderReleaseNoteEntries(entries: readonly ReleaseNoteEntry[]): string {
  return entries.map(renderReleaseNoteEntry).join("\n");
}

export function renderChangelogSection(
  release: ChangelogRelease,
  repository: RepositoryRef | null = null,
): string {
  return renderChangelogRelease(release, repository);
}

export function renderChangelogRelease(
  release: ChangelogRelease,
  repository: RepositoryRef | null = null,
): string {
  const lines = [`## [${release.version}] - ${release.date}`];
  const categories = groupEntriesByCategory(release.entries);

  for (const category of categoryOrder(categories)) {
    const entries = categories.get(category);
    if (entries === undefined || entries.length === 0) {
      continue;
    }

    lines.push("", `### ${category}`);

    for (const entry of entries) {
      lines.push("", renderChangelogEntry(entry, repository));
    }
  }

  return lines.join("\n");
}

export function renderChangelogDocument(
  document: ChangelogDocument,
  repository: RepositoryRef | null = null,
): string {
  const sections = [`# ${document.title}`, "", ...trimDocumentHeader(document.header)];

  for (const release of document.releases) {
    sections.push("", renderChangelogRelease(release, repository));
  }

  return sections.join("\n").trimEnd();
}

function renderReleaseNoteEntry(entry: ReleaseNoteEntry): string {
  if (entry.category === "Other") {
    return `- ${entry.text}`;
  }

  return `- ${entry.category}: ${entry.text}`;
}

function renderChangelogEntry(
  entry: ChangelogEntry,
  repository: RepositoryRef | null,
): string {
  const base = `- ${entry.text}`;

  if (entry.pullRequest === null || repository === null) {
    return base;
  }

  return `${base} ([#${entry.pullRequest.id}](https://github.com/${repository.owner}/${repository.repo}/pull/${entry.pullRequest.id}))`;
}

function groupEntriesByCategory(
  entries: readonly ChangelogEntry[],
): Map<ReleaseNoteCategory, ChangelogEntry[]> {
  const groups = new Map<ReleaseNoteCategory, ChangelogEntry[]>();

  for (const entry of entries) {
    const bucket = groups.get(entry.category);
    if (bucket === undefined) {
      groups.set(entry.category, [entry]);
    } else {
      bucket.push(entry);
    }
  }

  return groups;
}

function categoryOrder(
  groups: Map<ReleaseNoteCategory, ChangelogEntry[]>,
): ReleaseNoteCategory[] {
  const order: ReleaseNoteCategory[] = [...DEFAULT_RELEASE_NOTE_CATEGORIES];
  for (const category of groups.keys()) {
    if (!order.includes(category)) {
      order.push(category);
    }
  }
  return order;
}

function trimDocumentHeader(header: string): string[] {
  if (header.trim().length === 0) {
    return [];
  }

  return header.split(/\r?\n/);
}
