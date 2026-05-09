import type {
  ReleaseNoteEntry,
  ValidateReleaseNotesOptions,
  ValidateReleaseNotesResult,
} from "./struct.ts";

export function validateReleaseNotes(
  content: string,
  options: ValidateReleaseNotesOptions,
): ValidateReleaseNotesResult {
  const lines = content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (lines.length === 0) {
    return {
      kind: "invalid",
      errors: ["release note section must not be empty"],
    };
  }

  const allowedCategories = new Set(options.allowedCategories);
  const entries: ReleaseNoteEntry[] = [];
  const errors: string[] = [];
  let hasIgnoredItem = false;

  for (const line of lines) {
    const parsed = parseReleaseNoteLine(line, allowedCategories, options.fallbackCategory);

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
  fallbackCategory: "Other" | null,
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
    if (fallbackCategory === null) {
      return {
        kind: "invalid",
        error: "release note items without a category prefix require a fallback category",
      };
    }

    return {
      kind: "entry",
      entry: {
        category: fallbackCategory,
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
