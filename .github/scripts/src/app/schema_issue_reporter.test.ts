import { expect, test, describe, mock } from "bun:test";
import { reportSchemaInconsistency } from "./schema_issue_reporter";

describe("Schema Issue Reporter", () => {
  test("creates issue when inconsistencies are found", async () => {
    // 引数の型を明示的に指定してモックを作成
    const createIssueMock = mock((_params: { title: string; body: string; [key: string]: any }) =>
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
        error: mock(() => { })
      }
    } as any;

    const result = {
      missingInSchema: ["0.15.0"],
      extraInSchema: ["0.14.99"],
      missingInRequired: ["0.14.2"],
      extraInRequired: ["0.15.1"]
    };

    await reportSchemaInconsistency(args, result);

    expect(createIssueMock).toHaveBeenCalled();
    // 型アサーションを使用して title/body へのアクセスを可能にする
    const callArgs = createIssueMock.mock.calls[0]?.[0] as { title: string; body: string } | undefined;
    if (!callArgs) {
      throw new Error("Expected createIssueMock to be called with arguments");
    }

    // タイトルに件数が含まれているか
    expect(callArgs.title).toContain("2 issues"); // missingInSchema + missingInRequired
    expect(callArgs.title).toContain("2 extra");

    // 本文にバージョンが含まれているか
    expect(callArgs.body).toContain("0.14.99");
    expect(callArgs.body).toContain("0.14.2");
    expect(callArgs.body).toContain("0.15.1");
  });

  test("skips issue creation when no issues are found", async () => {
    const createIssueMock = mock(() => Promise.resolve({}));
    const args = {
      github: {
        rest: {
          issues: {
            create: createIssueMock
          }
        }
      },
      core: {
        info: mock(() => { })
      }
    } as any;

    const result = {
      missingInSchema: [],
      extraInSchema: [],
      missingInRequired: [],
      extraInRequired: []
    };

    await reportSchemaInconsistency(args, result);
    expect(createIssueMock).not.toHaveBeenCalled();
  });
});
