const BASELINE_CONTEXT = `You are working on a codebase. Make the requested changes using your native Read, Edit, and Write tools. Do not use jig.`;

const JIG_PATTERNS = [
  /^.*\bjig\s+(run|workflow|library|vars|validate|render)\b.*$/gm,
  /^.*\bjig\b.*\brecipe\b.*$/gm,
  /^.*--vars\b.*$/gm,
];

export function transformPromptForBaseline(prompt: string): string {
  let result = prompt;

  // Strip jig-specific references
  for (const pattern of JIG_PATTERNS) {
    result = result.replace(pattern, "").trim();
  }

  // Remove consecutive blank lines left by stripping
  result = result.replace(/\n{3,}/g, "\n\n");

  return `${BASELINE_CONTEXT}\n\n${result}`;
}
