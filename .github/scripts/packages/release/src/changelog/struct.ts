export const DEFAULT_RELEASE_NOTE_CATEGORIES = [
  "Added",
  "Changed",
  "Deprecated",
  "Removed",
  "Fixed",
  "Security",
  "Other",
] as const;

export type ReleaseNoteCategory = (typeof DEFAULT_RELEASE_NOTE_CATEGORIES)[number];

export type ReleaseNoteEntry = {
  category: string;
  text: string;
};

export type ExtractReleaseNotesSectionResult =
  | {
      kind: "found";
      content: string;
    }
  | {
      kind: "missing";
    };

export type ValidateReleaseNotesResult =
  | {
      kind: "entries";
      entries: ReleaseNoteEntry[];
    }
  | {
      kind: "ignored";
    }
  | {
      kind: "invalid";
      errors: string[];
    };

export type ValidateReleaseNotesOptions = {
  allowedCategories: readonly string[];
};
