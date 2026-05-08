import type { AsyncFunctionArguments } from "@actions/github-script";
import type { ToolchainUpdateResult } from "../monitor/toolchain_update_checker.ts";
import { buildToolchainUpdateIssueBody } from "../monitor/toolchain_update_body_builder.ts";

export async function reportToolchainUpdate(
  { github, context, core }: AsyncFunctionArguments,
  result: ToolchainUpdateResult,
  resolverDir: string
) {
  const issueCount = result.files.reduce((sum, file) => {
    return (
      sum +
      file.missingVersions.length +
      file.extraVersions.length +
      file.duplicateValueVersions.length +
      file.ignoredVersionsNotInReleases.length +
      file.ignoredVersionsPresentInValues.length +
      file.duplicateIgnoredVersions.length
    );
  }, 0);

  if (issueCount === 0) {
    core.info("No toolchain update issues found. Skipping issue creation.");
    return;
  }

  const title = `[Automation] Toolchain Update Monitor: ${result.files.length} file(s), ${issueCount} issue item(s)`;
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
