import { Command } from "commander";
import { normalizeVersion } from "./version/version_validator.ts";

const program = new Command();

program
  .name("typstlab-release")
  .description("Release automation helpers")
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

program.parse();
