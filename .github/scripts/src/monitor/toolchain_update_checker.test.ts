import { expect, test, describe } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { checkToolchainUpdate, type ToolchainUpdateResult } from "./toolchain_update_checker";

describe("Toolchain Update Checker", () => {
  test("detects version mismatches and ignore issues per resolver file", async () => {
    const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), "typstlab-workspace-"));
    const resolverDir = path.join(workspaceRoot, "crates/typstlab-base/src/version_resolver_jsons");
    fs.mkdirSync(resolverDir, { recursive: true });

    fs.writeFileSync(
      path.join(resolverDir, "typst.json"),
      JSON.stringify(
        {
          $schema: "./typst_version_schema.json",
          base_url: "https://github.com/typst/typst",
          version_pattern: "v{version}",
          ignores: ["0.13.1", "0.99.0", "0.13.1"],
          "0.14.3": ["0.14.3", "0.14.3", "0.99.0"],
          "0.14.1": ["0.14.1"]
        },
        null,
        2
      )
    );

    fs.writeFileSync(
      path.join(resolverDir, "type_docs.json"),
      JSON.stringify(
        {
          $schema: "./typst_version_schema.json",
          base_url: "https://github.com/typst-community/dev-builds",
          version_pattern: "docs-v{version}",
          "0.14.3": ["0.14.3", "0.14.1", "0.14.0"],
          "0.14.1": ["0.14.3", "0.14.1", "0.14.0"]
        },
        null,
        2
      )
    );

    fs.writeFileSync(
      path.join(resolverDir, "typst_version_schema.json"),
      JSON.stringify({ properties: {}, required: [] }, null, 2)
    );

    const github = {
      paginate: async (_fn: unknown, params: { owner: string; repo: string }) => {
        if (params.owner === "typst" && params.repo === "typst") {
          return [
            { tag_name: "v0.14.3" },
            { tag_name: "v0.14.1" },
            { tag_name: "v0.14.0" },
            { tag_name: "v0.13.1" },
            { tag_name: "v0.14.0-rc1" }
          ];
        }

        if (params.owner === "typst-community" && params.repo === "dev-builds") {
          return [
            { tag_name: "docs-v0.14.3" },
            { tag_name: "docs-v0.14.1" },
            { tag_name: "docs-v0.14.0" }
          ];
        }

        throw new Error(`Unexpected repository: ${params.owner}/${params.repo}`);
      },
      rest: {
        repos: {
          listReleases: () => undefined
        }
      }
    } as any;

    const result = await checkToolchainUpdate(
      { github } as any,
      resolverDir,
      workspaceRoot
    );

    expect(result.files).toHaveLength(1);
    expect(result.files[0]?.filePath).toBe("crates/typstlab-base/src/version_resolver_jsons/typst.json");
    expect(result.files[0]?.versionChecks).toHaveLength(2);
    expect(result.files[0]?.versionChecks[0]?.typstVersion).toBe("0.14.3");
    expect(result.files[0]?.versionChecks[0]?.missingVersions).toEqual(["0.14.1", "0.14.0"]);
    expect(result.files[0]?.versionChecks[0]?.extraVersions).toEqual([]);
    expect(result.files[0]?.versionChecks[0]?.duplicateVersions).toEqual(["0.14.3"]);
    expect(result.files[0]?.versionChecks[1]?.typstVersion).toBe("0.14.1");
    expect(result.files[0]?.versionChecks[1]?.missingVersions).toEqual(["0.14.3", "0.14.0"]);
    expect(result.files[0]?.versionChecks[1]?.extraVersions).toEqual([]);
    expect(result.files[0]?.versionChecks[1]?.duplicateVersions).toEqual([]);
    expect(result.files[0]?.ignoreCheck.extraVersions).toEqual(["0.99.0"]);
    expect(result.files[0]?.ignoreCheck.duplicateVersions).toEqual(["0.13.1"]);
  });

  test("returns empty result when all resolver files are synchronized", async () => {
    const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), "typstlab-workspace-"));
    const resolverDir = path.join(workspaceRoot, "crates/typstlab-base/src/version_resolver_jsons");
    fs.mkdirSync(resolverDir, { recursive: true });

    fs.writeFileSync(
      path.join(resolverDir, "typst.json"),
      JSON.stringify(
        {
          $schema: "./typst_version_schema.json",
          base_url: "https://github.com/typst/typst",
          version_pattern: "v{version}",
          "0.14.3": ["0.14.3", "0.14.1", "0.14.0"],
          "0.14.1": ["0.14.3", "0.14.1", "0.14.0"]
        },
        null,
        2
      )
    );

    fs.writeFileSync(
      path.join(resolverDir, "typst_version_schema.json"),
      JSON.stringify({ properties: {}, required: [] }, null, 2)
    );

    const github = {
      paginate: async () => [
        { tag_name: "v0.14.3" },
        { tag_name: "v0.14.1" },
        { tag_name: "v0.14.0" }
      ],
      rest: {
        repos: {
          listReleases: () => undefined
        }
      }
    } as any;

    const result: ToolchainUpdateResult = await checkToolchainUpdate(
      { github } as any,
      resolverDir,
      workspaceRoot
    );

    expect(result.files).toEqual([]);
  });
});
