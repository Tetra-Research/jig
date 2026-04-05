import type { Scenario } from "./types.ts";

const BASELINE_CONTEXT = `You are working on a codebase. Make the requested changes using your native Read, Edit, and Write tools. Do not use jig.`;

const JIG_PATTERNS = [
  /^.*\bjig\s+(run|workflow|library|vars|validate|render)\b.*$/gm,
  /^.*\bjig\b.*\brecipe\b.*$/gm,
  /^.*--vars\b.*$/gm,
];

export function transformPromptForBaseline(scenario: Scenario): string {
  let prompt = scenario.prompt;

  // Strip jig-specific references
  for (const pattern of JIG_PATTERNS) {
    prompt = prompt.replace(pattern, "").trim();
  }

  // Remove consecutive blank lines left by stripping
  prompt = prompt.replace(/\n{3,}/g, "\n\n");

  return `${BASELINE_CONTEXT}\n\n${prompt}`;
}
