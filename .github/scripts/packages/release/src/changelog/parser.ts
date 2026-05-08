import type {
  ExtractReleaseNotesSectionResult,
  ReleaseNoteEntry,
  ValidateReleaseNotesOptions,
  ValidateReleaseNotesResult,
} from "./struct.ts";

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

export function validateReleaseNotes(
  content: string,
  options: ValidateReleaseNotesOptions,
): ValidateReleaseNotesResult {
  const lines = content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const allowedCategories = new Set(options.allowedCategories);
  const entries: ReleaseNoteEntry[] = [];
  const errors: string[] = [];
  let hasIgnoredItem = false;

  for (const line of lines) {
    const parsed = parseReleaseNoteLine(line, allowedCategories);

    if (parsed.kind === "ignored") {
      hasIgnoredItem = true;
    } else if (parsed.kind === "entry") {
      entries.push(parsed.entry);
    } else {
      errors.push(parsed.error);
    }
  }

  if (hasIgnoredItem && entries.length > 0) {
    errors.unshift("- N/A must be the only release note item");
  }

  if (errors.length > 0) {
    return { kind: "invalid", errors };
  }

  if (hasIgnoredItem) {
    return { kind: "ignored" };
  }

  return { kind: "entries", entries };
}

type ParseReleaseNoteLineResult =
  | {
      kind: "entry";
      entry: ReleaseNoteEntry;
    }
  | {
      kind: "ignored";
    }
  | {
      kind: "invalid";
      error: string;
    };

function parseReleaseNoteLine(
  line: string,
  allowedCategories: ReadonlySet<string>,
): ParseReleaseNoteLineResult {
  if (!line.startsWith("- ")) {
    return {
      kind: "invalid",
      error: `release note line must start with '- ': ${line}`,
    };
  }

  const item = line.slice(2).trim();

  if (item === "N/A") {
    return { kind: "ignored" };
  }

  const categoryMatch = item.match(/^([^:]+):\s*(.*)$/);

  if (categoryMatch === null) {
    if (!allowedCategories.has("Other")) {
      return {
        kind: "invalid",
        error: "Other category is required for release note items without a category prefix",
      };
    }

    return {
      kind: "entry",
      entry: {
        category: "Other",
        text: item,
      },
    };
  }

  const category = categoryMatch[1];
  const text = categoryMatch[2];

  if (category === undefined || text === undefined) {
    return {
      kind: "invalid",
      error: `release note item must use '- Category: text': ${line}`,
    };
  }

  if (!allowedCategories.has(category)) {
    return {
      kind: "invalid",
      error: `unknown release note category: ${category}`,
    };
  }

  if (text.trim().length === 0) {
    return {
      kind: "invalid",
      error: `release note text must not be empty: ${line}`,
    };
  }

  return {
    kind: "entry",
    entry: {
      category,
      text: text.trim(),
    },
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
