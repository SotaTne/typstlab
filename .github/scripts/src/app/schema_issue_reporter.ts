import type { AsyncFunctionArguments } from "@actions/github-script";
import type { ConsistencyResult } from "../schema/typst_consistency";
import { buildConsistencyIssueBody } from "../schema/typst_consistency_body_builder";

/**
 * Expert in reporting schema inconsistencies via GitHub Issues.
 */
export async function reportSchemaInconsistency(
  { github, context, core }: AsyncFunctionArguments,
  result: ConsistencyResult,
  schemaPath: string
) {
  const missingCount = result.missingInSchema.length + result.missingInRequired.length;
  const extraCount = result.extraInSchema.length + result.extraInRequired.length;
  const ignoredCount = result.ignoredInProperties.length + result.ignoredInRequired.length;

  if (missingCount === 0 && extraCount === 0 && ignoredCount === 0) {
    core.info("No inconsistencies found. Skipping issue creation.");
    return;
  }

  const title = `[Automation] Schema Consistency: ${missingCount} issues, ${extraCount} extra versions found${ignoredCount > 0 ? `, ${ignoredCount} ignored-version hits` : ""}`;
  
  // 本文をスキーマ側の専門家に作ってもらう
  const body = buildConsistencyIssueBody(result, schemaPath);

  core.info(`Creating issue: ${title}`);

  const { data: issue } = await github.rest.issues.create({
    owner: context.repo.owner,
    repo: context.repo.repo,
    title,
    body,
    labels: ["maintenance", "automation"],
  });

  core.info(`Successfully created issue: ${issue.html_url}`);
}
