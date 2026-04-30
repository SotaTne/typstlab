import { expect, test, describe, mock } from "bun:test";
import { reportToolchainUpdate } from "./toolchain_update_reporter";

describe("Toolchain Update Reporter", () => {
  test("creates issue for toolchain mismatches", async () => {
    const createIssueMock = mock((_params: { title: string; body: string; [key: string]: unknown }) =>
      Promise.resolve({ data: { html_url: "https://github.com/mock/issue/1" } })
    );

    const args = {
      github: {
        rest: {
          issues: {
            create: createIssueMock
          }
        }
      },
      context: {
        repo: {
          owner: "test-owner",
          repo: "test-repo"
        }
      },
      core: {
        info: mock(() => { }),
      }
    } as any;

    const result = {
      files: [
        {
          filePath: "crates/typstlab-base/src/version_resolver_jsons/typst.json",
          baseUrl: "https://github.com/typst/typst",
          versionPattern: "v{version}",
          releaseVersions: ["0.14.3", "0.14.1", "0.14.0"],
          ignoredVersions: ["0.13.1"],
          versionChecks: [
            {
              typstVersion: "0.14.1",
              missingVersions: ["0.14.0"],
              extraVersions: [],
              duplicateVersions: []
            }
          ],
          ignoreCheck: {
            extraVersions: ["0.99.0"],
            duplicateVersions: ["0.13.1"]
          }
        }
      ]
    };

    await reportToolchainUpdate(args, result as any, "crates/typstlab/src/version_resolver_jsons");

    expect(createIssueMock).toHaveBeenCalled();
    const callArgs = createIssueMock.mock.calls[0]?.[0] as { title: string; body: string } | undefined;
    if (!callArgs) {
      throw new Error("Expected createIssueMock to be called with arguments");
    }

    expect(callArgs.title).toContain("Toolchain Update Monitor");
    expect(callArgs.title).toContain("1 file(s)");
    expect(callArgs.title).toContain("1 version issue(s)");
    expect(callArgs.title).toContain("2 ignore issue(s)");
    expect(callArgs.body).toContain("# 🔍 Toolchain Update Report");
    expect(callArgs.body).toContain("## typst/typst");
    expect(callArgs.body).toContain("Path: `crates/typstlab-base/src/version_resolver_jsons/typst.json`");
    expect(callArgs.body).toContain("Base URL: `https://github.com/typst/typst`");
    expect(callArgs.body).toContain("Version pattern: `v{version}`");
    expect(callArgs.body).toContain("##### `0.14.1`");
    expect(callArgs.body).toContain("###### ❌ Missing versions");
    expect(callArgs.body).toContain("`0.14.0`");
    expect(callArgs.body).toContain("##### `ignores`");
  });

  test("skips issue creation when no toolchain issues are found", async () => {
    const createIssueMock = mock(() => Promise.resolve({}));
    const args = {
      github: {
        rest: {
          issues: {
            create: createIssueMock
          }
        }
      },
      context: {
        repo: {
          owner: "test-owner",
          repo: "test-repo"
        }
      },
      core: {
        info: mock(() => { })
      }
    } as any;

    await reportToolchainUpdate(args, { files: [] }, "crates/typstlab-base/src/version_resolver_jsons");

    expect(createIssueMock).not.toHaveBeenCalled();
  });
});
