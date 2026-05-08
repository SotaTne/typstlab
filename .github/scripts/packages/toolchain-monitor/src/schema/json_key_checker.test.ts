import { expect, test, describe } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { checkJsonFileKeys } from "./json_key_checker";

describe("JSON Key Checker", () => {
  test("detects missing and extra version keys with relative file paths", () => {
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
          "0.14.2": ["0.14.2"],
          "0.99.0": ["0.99.0"]
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
          "0.14.2": ["0.14.2"]
        },
        null,
        2
      )
    );

    fs.writeFileSync(
      path.join(resolverDir, "typst_version_schema.json"),
      JSON.stringify({ properties: {}, required: [] }, null, 2)
    );

    const result = checkJsonFileKeys(resolverDir, workspaceRoot, ["0.14.2", "0.14.1"]);

    expect(result.files).toHaveLength(2);
    expect(result.files[0]?.filePath).toBe("crates/typstlab-base/src/version_resolver_jsons/type_docs.json");
    expect(result.files[0]?.missingKeys).toEqual(["0.14.1"]);
    expect(result.files[0]?.extraKeys).toEqual([]);
    expect(result.files[1]?.filePath).toBe("crates/typstlab-base/src/version_resolver_jsons/typst.json");
    expect(result.files[1]?.missingKeys).toEqual(["0.14.1"]);
    expect(result.files[1]?.extraKeys).toEqual(["0.99.0"]);
  });

  test("returns empty result when files are already synchronized", () => {
    const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), "typstlab-workspace-"));
    const resolverDir = path.join(workspaceRoot, "crates/typstlab-base/src/version_resolver_jsons");
    fs.mkdirSync(resolverDir, { recursive: true });

    fs.writeFileSync(
      path.join(resolverDir, "typst.json"),
      JSON.stringify(
        {
          "0.14.2": ["0.14.2"],
          "0.14.1": ["0.14.1"]
        },
        null,
        2
      )
    );

    const result = checkJsonFileKeys(resolverDir, workspaceRoot, ["0.14.2", "0.14.1"]);

    expect(result.files).toEqual([]);
  });
});
