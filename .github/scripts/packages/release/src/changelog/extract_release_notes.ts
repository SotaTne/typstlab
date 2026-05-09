import type { ExtractReleaseNotesSectionResult } from "./struct.ts";

export function extractReleaseNotesSection(
  body: string,
  heading: string,
): ExtractReleaseNotesSectionResult {
  const lines = body.split(/\r?\n/);
  const headingLine = `## ${heading}`;
  const start = lines.findIndex((line) => line.trim() === headingLine);

  if (start === -1) {
    return { kind: "missing" };
  }

  const sectionLines = lines.slice(start + 1);
  const end = sectionLines.findIndex((line) => line.trim().startsWith("## "));
  const contentLines = end === -1 ? sectionLines : sectionLines.slice(0, end);

  return {
    kind: "found",
    content: trimBlankLines(contentLines).join("\n"),
  };
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
