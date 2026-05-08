import { expect, test, describe } from "bun:test";
import { extractVersion, isNewer } from "./version_extractor";

describe("Version Extractor", () => {
  test("extracts version using simple pattern", () => {
    expect(extractVersion("v0.14.2", "v{version}")).toBe("0.14.2");
  });

  test("extracts version using complex pattern", () => {
    expect(extractVersion("docs-v0.14.2", "docs-v{version}")).toBe("0.14.2");
  });

  test("returns null if pattern does not match", () => {
    expect(extractVersion("v0.14.2", "docs-v{version}")).toBe(null);
  });
});

describe("isNewer", () => {
  test("compares semver correctly", () => {
    expect(isNewer("0.12.0", "0.14.2")).toBe(true);
    expect(isNewer("0.14.2", "0.14.2")).toBe(false);
    expect(isNewer("0.14.2", "0.14.1")).toBe(false);
  });
});
