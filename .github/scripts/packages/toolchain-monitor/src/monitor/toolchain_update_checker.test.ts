import { expect, test, describe } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { checkToolchainUpdate, type ToolchainUpdateResult } from "./toolchain_update_checker";

describe("Toolchain Update Checker", () => {
  test("detects aggregated mismatches, duplicate assignments, and ignore issues", async () => {
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
          ignores: ["0.14.0", "0.99.0", "0.14.0"],
          "0.14.3": ["0.14.3", "0.14.2"],
          "0.14.2": ["0.14.2", "0.13.0"],
          "0.14.1": ["0.14.0"]
        },
        null,
        2
      )
    );

    fs.writeFileSync(
      path.join(resolverDir, "typstyle.json"),
      JSON.stringify(
        {
          $schema: "./typst_version_schema.json",
          base_url: "https://github.com/typstyle-rs/typstyle",
          version_pattern: "v{version}",
          "0.14.3": ["0.14.3"],
          "0.14.2": ["0.14.2"],
          "0.14.1": ["0.14.1"],
          "0.14.0": ["0.14.0"]
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
        const releaseSets: Record<string, Array<{ tag_name: string }>> = {
          "typst/typst": [
            { tag_name: "v0.14.3" },
            { tag_name: "v0.14.2" },
            { tag_name: "v0.14.1" },
            { tag_name: "v0.14.0" }
          ],
          "typstyle-rs/typstyle": [
            { tag_name: "v0.14.3" },
            { tag_name: "v0.14.2" },
            { tag_name: "v0.14.1" },
            { tag_name: "v0.14.0" }
          ]
        };

        const key = `${params.owner}/${params.repo}`;
        const releases = releaseSets[key];
        if (!releases) {
          throw new Error(`Unexpected repository: ${key}`);
        }

        return releases;
      },
      rest: {
        repos: {
          listReleases: () => undefined
        }
      }
    } as any;

    const result: ToolchainUpdateResult = await checkToolchainUpdate({ github } as any, resolverDir, workspaceRoot);

    expect(result.files).toHaveLength(1);

    const file = result.files[0];
    expect(file?.filePath).toBe("crates/typstlab-base/src/version_resolver_jsons/typst.json");
    expect(file?.repoName).toBe("typst/typst");
    expect(file?.releaseVersions).toEqual(["0.14.3", "0.14.2", "0.14.1", "0.14.0"]);
    expect(file?.ignoredVersions).toEqual(["0.14.0", "0.99.0"]);
    expect(file?.missingVersions).toEqual(["0.14.1"]);
    expect(file?.extraVersions).toEqual(["0.13.0"]);
    expect(file?.duplicateValueVersions).toEqual([
      {
        version: "0.14.2",
        assignments: [
          { typstVersion: "0.14.3", count: 1 },
          { typstVersion: "0.14.2", count: 1 }
        ]
      }
    ]);
    expect(file?.ignoredVersionsNotInReleases).toEqual(["0.99.0"]);
    expect(file?.ignoredVersionsPresentInValues).toEqual(["0.14.0"]);
    expect(file?.duplicateIgnoredVersions).toEqual(["0.14.0"]);
  });

  test("returns empty result when every file is synchronized", async () => {
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
          "0.14.3": ["0.14.3"],
          "0.14.2": ["0.14.2"],
          "0.14.1": ["0.14.1"],
          "0.14.0": ["0.14.0"]
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
        { tag_name: "v0.14.2" },
        { tag_name: "v0.14.1" },
        { tag_name: "v0.14.0" }
      ],
      rest: {
        repos: {
          listReleases: () => undefined
        }
      }
    } as any;

    const result: ToolchainUpdateResult = await checkToolchainUpdate({ github } as any, resolverDir, workspaceRoot);

    expect(result.files).toEqual([]);
  });
});
