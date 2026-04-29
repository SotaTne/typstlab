import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { Command } from "commander";
import * as core from "@actions/core";
import * as github from "@actions/github";
import * as exec from "@actions/exec";
import * as glob from "@actions/glob";
import * as io from "@actions/io";
import * as jobs from "./index";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const program = new Command();

program
  .name("typstlab-scripts")
  .description("CLI for running GitHub Action scripts locally")
  .version("1.0.0")
  .option("-t, --token <string>", "GitHub Token", process.env.GITHUB_TOKEN)
  .option("-r, --repo <string>", "GitHub Repository (owner/repo)", process.env.GITHUB_REPOSITORY || "SotaTne/typstlab");

/**
 * 必要な環境変数のチェックとセットアップ
 */
function validateAndSetupEnv(opts: any) {
  if (!opts.token) {
    console.error("\x1b[31mError: GITHUB_TOKEN is required.\x1b[0m");
    console.error("Please set GITHUB_TOKEN environment variable or use -t option.");
    process.exit(1);
  }

  if (!opts.repo || !opts.repo.includes("/")) {
    console.error("\x1b[31mError: GITHUB_REPOSITORY (owner/repo) is required.\x1b[0m");
    console.error("Please set GITHUB_REPOSITORY environment variable or use -r option.");
    process.exit(1);
  }

  // 本物のモジュールが期待する環境変数をセット
  process.env.GITHUB_TOKEN = opts.token;
  process.env.GITHUB_REPOSITORY = opts.repo;
  if (!process.env.GITHUB_WORKSPACE) {
    process.env.GITHUB_WORKSPACE = path.resolve(__dirname, "../../../");
  }
}

/**
 * @actions/github-script の引数構造を本物のモジュールで組み立てる
 */
function setupArgs(token: string) {
  return {
    github: github.getOctokit(token),
    context: github.context,
    core: core,
    exec: exec,
    glob: glob,
    io: io,
    require: require,
    __original_require__: require,
    getOctokit: github.getOctokit,
  };
}

program
  .command("list")
  .description("List all available jobs")
  .action(() => {
    const availableJobs = Object.keys(jobs).filter((k) => k.startsWith("job"));
    console.log("\x1b[1mAvailable Jobs:\x1b[0m");
    availableJobs.forEach((j) => console.log(` - ${j}`));
  });

program
  .command("run <jobName>")
  .description("Run a specific job using real @actions modules")
  .action(async (jobName) => {
    const opts = program.opts();
    validateAndSetupEnv(opts);

    const job = (jobs as any)[jobName];
    if (!job || typeof job !== "function") {
      console.error(`\x1b[31mError: Job "${jobName}" not found in index.ts.\x1b[0m`);
      process.exit(1);
    }

    console.log(`🚀 Running job: \x1b[1m${jobName}\x1b[0m on \x1b[1m${opts.repo}\x1b[0m...\n`);

    try {
      const args = setupArgs(opts.token);
      await job(args);
      console.log(`\n✨ Job \x1b[1m${jobName}\x1b[0m finished.`);
    } catch (err) {
      console.error(`\n💥 Error during \x1b[1m${jobName}\x1b[0m:`);
      console.error(err);
      process.exit(1);
    }
  });

program.parse();
