import { describe, expect, test } from "bun:test";
import {
  isStableVersion,
  isVersionTag,
  normalizeVersion,
  parseStableVersion,
} from "./version_validator.ts";

describe("parseStableVersion", () => {
  test("accepts plain stable versions", () => {
    expect(parseStableVersion("1.2.3")).toEqual({
      major: 1,
      minor: 2,
      patch: 3,
    });
  });

  test("accepts v-prefixed stable version tags", () => {
    expect(parseStableVersion("v1.2.3")).toEqual({
      major: 1,
      minor: 2,
      patch: 3,
    });
  });

  test("rejects unstable or malformed versions", () => {
    expect(parseStableVersion("1.2")).toBeNull();
    expect(parseStableVersion("1.2.3-alpha.1")).toBeNull();
    expect(parseStableVersion("version-1.2.3")).toBeNull();
  });
});

describe("isVersionTag", () => {
  test("requires a v prefix", () => {
    expect(isVersionTag("v1.2.3")).toBe(true);
    expect(isVersionTag("1.2.3")).toBe(false);
  });
});

describe("normalizeVersion", () => {
  test("removes the v prefix", () => {
    expect(normalizeVersion("v1.2.3")).toBe("1.2.3");
  });

  test("throws for invalid versions", () => {
    expect(() => normalizeVersion("v1.2")).toThrow("expected stable version");
  });
});

describe("isStableVersion", () => {
  test("accepts x.y.z and vx.y.z", () => {
    expect(isStableVersion("1.2.3")).toBe(true);
    expect(isStableVersion("v1.2.3")).toBe(true);
  });
});
