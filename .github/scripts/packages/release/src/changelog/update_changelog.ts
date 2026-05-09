import type { ChangelogDocument, ChangelogRelease } from "./struct.ts";

export function upsertChangelogRelease(
  document: ChangelogDocument,
  release: ChangelogRelease,
): ChangelogDocument {
  const remainingReleases = document.releases.filter((item) => item.version !== release.version);

  return {
    ...document,
    releases: [release, ...remainingReleases],
  };
}
