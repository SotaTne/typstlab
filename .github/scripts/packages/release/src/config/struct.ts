import type { AllowedReleaseNoteCategory } from "../changelog/struct.ts";

export type ReleaseConfig = {
  changelogPath: string;
  changelogTitle: string;
  changelogHeader: string;
  releaseNotesHeading: string;
  allowedCategories: readonly AllowedReleaseNoteCategory[];
  fallbackCategory: "Other" | null;
  releaseBranchPattern: string;
  releaseTagPattern: string;
  releasePrTitlePattern: string;
  releasePrLabel: string;
  releaseDraft: boolean;
};

export type ValidateReleaseConfigResult =
  | {
      kind: "valid";
      config: ReleaseConfig;
    }
  | {
      kind: "invalid";
      errors: string[];
    };
