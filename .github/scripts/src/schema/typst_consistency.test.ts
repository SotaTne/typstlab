import { expect, test, describe } from "bun:test";
import { checkTypstSchemaConsistency, type GitHubReleases } from "./typst_consistency";

describe("Typst Schema Consistency", () => {
  const mockReleases: GitHubReleases = [
    { tag_name: "v0.14.2" },
    { tag_name: "v0.14.1" },
    { tag_name: "v0.14.0-rc1" }, // 安定版でないので無視されるべき
  ] as unknown as GitHubReleases;

  test("identifies missing versions in schema", () => {
    const schema = {
      properties: {
        "0.14.1": {}
      },
      required: ["0.14.1"],
      version_ignores: []
    };
    const result = checkTypstSchemaConsistency(schema, mockReleases);
    expect(result.missingInSchema).toEqual(["0.14.2"]);
    expect(result.extraInSchema).toEqual([]);
  });

  test("identifies extra versions in schema", () => {
    const schema = {
      properties: {
        "0.14.2": {},
        "0.14.1": {},
        "0.15.0": {} // GHにないので過剰
      },
      required: ["0.14.2", "0.14.1", "0.15.0"],
      version_ignores: []
    };
    const result = checkTypstSchemaConsistency(schema, mockReleases);
    expect(result.extraInSchema).toEqual(["0.15.0"]);
  });

  test("identifies missing required versions", () => {
    const schema = {
      properties: {
        "0.14.2": {},
        "0.14.1": {}
      },
      required: ["0.14.2"], // 0.14.1 が漏れている
      version_ignores: []
    };
    const result = checkTypstSchemaConsistency(schema, mockReleases);
    expect(result.missingInRequired).toEqual(["0.14.1"]);
  });

  test("identifies extra versions in required", () => {
    const schema = {
      properties: {
        "0.14.2": {},
        "0.14.1": {}
      },
      required: ["0.14.2", "0.14.1", "0.15.0"],
      version_ignores: []
    };
    const result = checkTypstSchemaConsistency(schema, mockReleases);
    expect(result.extraInRequired).toEqual(["0.15.0"]);
  });

  test("ignores configured versions in release comparison and reports schema hits", () => {
    const schema = {
      properties: {
        "0.14.2": {},
        "0.14.1": {},
        "0.11.1": {}
      },
      required: ["0.14.2", "0.14.1", "0.11.1"],
      version_ignores: ["0.11.1"]
    };
    const result = checkTypstSchemaConsistency(schema, mockReleases);
    expect(result.missingInSchema).toEqual([]);
    expect(result.extraInSchema).toEqual([]);
    expect(result.missingInRequired).toEqual([]);
    expect(result.extraInRequired).toEqual([]);
    expect(result.ignoredInProperties).toEqual(["0.11.1"]);
    expect(result.ignoredInRequired).toEqual(["0.11.1"]);
  });
});
