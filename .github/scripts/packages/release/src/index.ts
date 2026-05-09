export {
  isStableVersion,
  isVersionTag,
  normalizeVersion,
  parseStableVersion,
  type StableVersion,
} from "./version/version_validator.ts";

export {
  extractChangelogReleaseSection,
  parseChangelogDocument,
} from "./changelog/extract_changelog_section.ts";
export {
  extractReleaseNotesSection,
} from "./changelog/extract_release_notes.ts";
export { validateReleaseNotes } from "./changelog/validate_release_notes.ts";
export {
  guardChangelogUpdate,
  hasChangelogPathChange,
} from "./changelog/guard_changelog_update.ts";
export {
  type ChangelogDocument,
  type ChangelogEntry,
  type ChangelogRelease,
  DEFAULT_RELEASE_NOTE_CATEGORIES,
  type ExtractChangelogSectionResult,
  type ExtractReleaseNotesSectionResult,
  type GuardChangelogUpdateOptions,
  type GuardChangelogUpdateResult,
  type ParseChangelogDocumentResult,
  type PullRequestRef,
  type ReleaseNoteCategory,
  type ReleaseNoteEntry,
  type RepositoryRef,
  type ValidateReleaseNotesOptions,
  type ValidateReleaseNotesResult,
} from "./changelog/struct.ts";
export {
  renderChangelogDocument,
  renderChangelogRelease,
  renderChangelogSection,
  renderReleaseNoteEntries,
} from "./changelog/render_changelog_section.ts";
export { upsertChangelogRelease } from "./changelog/update_changelog.ts";
