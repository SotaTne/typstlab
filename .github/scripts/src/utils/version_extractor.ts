/**
 * Pattern-based version extractor.
 * Converts a pattern like 'v{version}' or 'docs-v{version}' into a regex
 * and extracts the string at the {version} placeholder.
 */
export function extractVersion(tagName: string, pattern: string): string | null {
  // Escape special regex characters except for {version}
  const escapedPattern = pattern.replace(/[.+?^${}()|[\]\\]/g, (m) => {
    if (m === '{' || m === '}') return m;
    return '\\' + m;
  });

  // Replace {version} with a capture group
  const regexStr = `^${escapedPattern.replace('{version}', '(.*)')}$`;
  const regex = new RegExp(regexStr);

  const match = tagName.match(regex);
  return (match && match[1]) ? match[1] : null;
}

/**
 * Basic semver comparison (vX.Y.Z).
 * Returns true if latest is newer than current.
 */
export function isNewer(current: string, latest: string): boolean {
  const normalize = (v: string) => v.replace(/^v/, "").split(".").map(Number);
  const c = normalize(current);
  const l = normalize(latest);

  for (let i = 0; i < Math.max(c.length, l.length); i++) {
    const cv = c[i] || 0;
    const lv = l[i] || 0;
    if (lv > cv) return true;
    if (lv < cv) return false;
  }
  return false;
}
