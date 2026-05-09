import { readFile } from "node:fs/promises";
import { Command } from "commander";
import { loadReleaseConfig } from "./config/load_config.ts";
import { analyzePullRequestForRelease } from "./commands/pr_check.ts";
import { normalizeVersion } from "./version/version_validator.ts";

const program = new Command();

program
  .name("typstlab-release")
  .description("Release automation helpers")
  .version("1.0.0");

program
  .command("validate-version <version>")
  .description("Validate a stable semver version or v-prefixed tag")
  .action((version: string) => {
    try {
      console.log(normalizeVersion(version));
    } catch (error) {
      console.error(error instanceof Error ? error.message : String(error));
      process.exit(1);
    }
  });

program
  .command("show-config")
  .description("Load and print the release config from env as JSON")
  .action(() => {
    const result = loadReleaseConfig();

    if (result.kind === "invalid") {
      for (const error of result.errors) {
        console.error(error);
      }
      process.exit(1);
    }

    console.log(JSON.stringify(result.config, null, 2));
  });

program
  .command("pr-check")
  .description("Analyze a pull request snapshot for release automation rules")
  .requiredOption("-i, --input <file>", "Path to a JSON file containing the PR snapshot")
  .action(async (options: { input: string }) => {
    const config = loadReleaseConfig();

    if (config.kind === "invalid") {
      for (const error of config.errors) {
        console.error(error);
      }
      process.exit(1);
    }

    const input = await readFile(options.input, "utf-8");
    const pullRequest = JSON.parse(input) as {
      body: string;
      changedPaths: readonly string[];
      headRefName: string;
      title: string;
      labels: readonly string[];
    };

    const result = analyzePullRequestForRelease(config.config, pullRequest);
    console.log(JSON.stringify(result, null, 2));

    if (result.kind === "failure") {
      process.exit(1);
    }
  });

await program.parseAsync();
