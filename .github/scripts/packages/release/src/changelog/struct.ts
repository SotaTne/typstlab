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

export type RepositoryRef = {
  owner: string;
  repo: string;
};

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

export type ExtractChangelogSectionResult =
  | {
      kind: "found";
      content: string;
    }
  | {
      kind: "missing";
    }
  | {
      kind: "invalid";
      error: string;
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

export type PullRequestRef = {
  id: number;
};

export type ChangelogEntry = {
  category: ReleaseNoteCategory;
  text: string;
  pullRequest: PullRequestRef | null;
};

export type ChangelogRelease = {
  version: string;
  date: string;
  entries: ChangelogEntry[];
};

export type ChangelogDocument = {
  title: string;
  header: string;
  releases: ChangelogRelease[];
};

export type ParseChangelogDocumentResult =
  | {
      kind: "parsed";
      document: ChangelogDocument;
    }
  | {
      kind: "invalid";
      error: string;
    };

export type GuardChangelogUpdateOptions = {
  changedPaths: readonly string[];
  changelogPath: string;
  isReleasePr: boolean;
};

export type GuardChangelogUpdateResult =
  | {
      kind: "allowed";
    }
  | {
      kind: "blocked";
      reason: string;
    };
