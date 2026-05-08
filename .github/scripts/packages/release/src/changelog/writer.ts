import type { ReleaseNoteEntry } from "./struct.ts";

export function renderReleaseNoteEntries(entries: readonly ReleaseNoteEntry[]): string {
  return entries.map(renderReleaseNoteEntry).join("\n");
}

function renderReleaseNoteEntry(entry: ReleaseNoteEntry): string {
  if (entry.category === "Other") {
    return `- ${entry.text}`;
  }

  return `- ${entry.category}: ${entry.text}`;
}
