export type StableVersion = {
  major: number;
  minor: number;
  patch: number;
};

const STABLE_VERSION_PATTERN = /^(?:v)?([0-9]+)\.([0-9]+)\.([0-9]+)$/;

export function parseStableVersion(version: string): StableVersion | null {
  const match = version.match(STABLE_VERSION_PATTERN);
  if (!match) {
    return null;
  }

  const [, major, minor, patch] = match;
  return {
    major: Number(major),
    minor: Number(minor),
    patch: Number(patch),
  };
}

export function isStableVersion(version: string): boolean {
  return parseStableVersion(version) !== null;
}

export function isVersionTag(tag: string): boolean {
  return tag.startsWith("v") && parseStableVersion(tag) !== null;
}

export function normalizeVersion(version: string): string {
  const parsed = parseStableVersion(version);
  if (!parsed) {
    throw new Error(`expected stable version x.y.z or vx.y.z, got: ${version}`);
  }

  return `${parsed.major}.${parsed.minor}.${parsed.patch}`;
}
