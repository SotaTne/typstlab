export type ReleaseConfig = {
  changelogPath: string;
  changelogTitle: string;
  changelogHeader: string;
  releaseNotesHeading: string;
  allowedCategories: readonly string[];
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
