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
        info: mock(() => { })
      }
    } as any;

    const result = {
      files: [
        {
          filePath: "crates/typstlab-base/src/version_resolver_jsons/typst.json",
          repoName: "typst/typst",
          baseUrl: "https://github.com/typst/typst",
          versionPattern: "v{version}",
          releaseVersions: ["0.14.3", "0.14.2", "0.14.1", "0.14.0"],
          ignoredVersions: ["0.14.0", "0.99.0"],
          missingVersions: ["0.14.1"],
          extraVersions: ["0.13.0"],
          duplicateValueVersions: [
            {
              version: "0.14.2",
              assignments: [
                { typstVersion: "0.14.3", count: 1 },
                { typstVersion: "0.14.2", count: 1 }
              ]
            }
          ],
          ignoredVersionsNotInReleases: ["0.99.0"],
          ignoredVersionsPresentInValues: ["0.14.0"],
          duplicateIgnoredVersions: ["0.14.0"]
        }
      ]
    };

    await reportToolchainUpdate(args, result as any, "crates/typstlab-base/src/version_resolver_jsons");

    expect(createIssueMock).toHaveBeenCalled();
    const callArgs = createIssueMock.mock.calls[0]?.[0] as { title: string; body: string } | undefined;
    if (!callArgs) {
      throw new Error("Expected createIssueMock to be called with arguments");
    }

    expect(callArgs.title).toContain("Toolchain Update Monitor");
    expect(callArgs.title).toContain("1 file(s)");
    expect(callArgs.title).toContain("6 issue item(s)");
    expect(callArgs.body).toContain("# 🔍 Toolchain Update Report");
    expect(callArgs.body).toContain("## typst/typst");
    expect(callArgs.body).toContain("Path: `crates/typstlab-base/src/version_resolver_jsons/typst.json`");
    expect(callArgs.body).toContain("### Missing versions");
    expect(callArgs.body).toContain("### Extra versions");
    expect(callArgs.body).toContain("### Duplicate versions in JSON values");
    expect(callArgs.body).toContain("### Ignored versions still present in JSON values");
    expect(callArgs.body).toContain("### Ignored versions not found in releases");
    expect(callArgs.body).toContain("### Duplicate ignores");
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
