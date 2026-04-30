import type { AsyncFunctionArguments } from "@actions/github-script";
import type { ToolchainUpdateResult } from "../monitor/toolchain_update_checker";
import { buildToolchainUpdateIssueBody } from "../monitor/toolchain_update_body_builder";

export async function reportToolchainUpdate(
  { github, context, core }: AsyncFunctionArguments,
  result: ToolchainUpdateResult,
  resolverDir: string
) {
  const versionIssueCount = result.files.reduce(
    (sum, file) =>
      sum +
      file.versionChecks.reduce(
        (fileSum, check) =>
          fileSum + check.missingVersions.length + check.extraVersions.length + check.duplicateVersions.length,
        0
      ),
    0
  );
  const ignoreIssueCount = result.files.reduce(
    (sum, file) => sum + file.ignoreCheck.extraVersions.length + file.ignoreCheck.duplicateVersions.length,
    0
  );

  if (versionIssueCount === 0 && ignoreIssueCount === 0) {
    core.info("No toolchain update issues found. Skipping issue creation.");
    return;
  }

  const title = `[Automation] Toolchain Update Monitor: ${result.files.length} file(s), ${versionIssueCount} version issue(s), ${ignoreIssueCount} ignore issue(s)`;
  const body = buildToolchainUpdateIssueBody(result.files);

  core.info(`Creating issue: ${title}`);
  core.info(`Source file group: ${resolverDir}`);

  const { data: issue } = await github.rest.issues.create({
    owner: context.repo.owner,
    repo: context.repo.repo,
    title,
    body,
    labels: ["maintenance", "automation"],
  });

  core.info(`Successfully created issue: ${issue.html_url}`);
}
