import fs from "node:fs";
import path from "node:path";
import { readdirRecursive } from "../lib/fs.ts";
import type {
  AssertionResult,
  JigInvocation,
  NegativeAssertionResult,
  Scenario,
  TrialScore,
} from "./types.ts";

export function scoreAssertions(scenario: Scenario, workDir: string): AssertionResult[] {
  return scenario.assertions.map((assertion) => {
    const filePath = path.join(workDir, assertion.file);
    if (!fs.existsSync(filePath)) {
      return { ...assertion, passed: false };
    }

    let content = fs.readFileSync(filePath, "utf-8");

    if (assertion.scope) {
      content = extractScope(content, assertion.scope);
    }

    const passed = content.includes(assertion.contains);
    return { ...assertion, passed };
  });
}

export function scoreNegativeAssertions(
  scenario: Scenario,
  workDir: string
): { passed: boolean; results: NegativeAssertionResult[] } {
  if (!scenario.negative_assertions || scenario.negative_assertions.length === 0) {
    return { passed: true, results: [] };
  }

  const results: NegativeAssertionResult[] = [];

  for (const na of scenario.negative_assertions) {
    if (na.any_file) {
      // Check all files in workDir
      const allFiles = readdirRecursive(workDir, { skipGit: true });
      let found = false;
      const re = new RegExp(na.not_contains);
      for (const f of allFiles) {
        const content = fs.readFileSync(f, "utf-8");
        if (re.test(content)) {
          found = true;
          break;
        }
      }
      results.push({
        any_file: true,
        not_contains: na.not_contains,
        passed: !found,
        description: na.description,
      });
    } else if (na.file) {
      const filePath = path.join(workDir, na.file);
      if (!fs.existsSync(filePath)) {
        // File doesn't exist, so it can't contain the forbidden string
        results.push({ file: na.file, not_contains: na.not_contains, passed: true, description: na.description });
      } else {
        const content = fs.readFileSync(filePath, "utf-8");
        const found = new RegExp(na.not_contains).test(content);
        results.push({ file: na.file, not_contains: na.not_contains, passed: !found, description: na.description });
      }
    }
  }

  return { passed: results.every((r) => r.passed), results };
}

export function scoreJigUsage(
  agentOutput: string,
  scenario: Scenario
): { jig_used: boolean; jig_correct: boolean; call_count: number; invocations: JigInvocation[] } {
  const invocations: JigInvocation[] = [];

  // Extract all text content from agent output (handles both plain text and stream-json)
  const searchText = extractAllText(agentOutput);

  for (const line of searchText.split("\n")) {
    const cmdMatch = line.match(/\bjig\s+(run|workflow|render)\s+(\S+)/);
    if (!cmdMatch) continue;

    const command = `jig ${cmdMatch[1]} ${cmdMatch[2]}`;
    let vars: string | undefined;
    const varsMatch = line.match(/--vars\s+(?:'([^']+)'|"([^"]+)")/);
    if (varsMatch) vars = varsMatch[1] ?? varsMatch[2];

    invocations.push({ command, vars });
  }

  const call_count = invocations.length;
  const jig_used = call_count > 0;

  // Check correctness: all vars must be valid JSON, call_count within limits
  let jig_correct = jig_used;
  if (jig_used) {
    for (const inv of invocations) {
      if (inv.vars) {
        try {
          JSON.parse(inv.vars);
        } catch {
          jig_correct = false;
          break;
        }
      }
    }
    if (scenario.max_jig_commands && call_count > scenario.max_jig_commands) {
      jig_correct = false;
    }
  }

  return { jig_used, jig_correct, call_count, invocations };
}

export interface EfficiencyMetrics {
  tool_calls: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_input_tokens: number;
  cache_read_input_tokens: number;
  tokens_used: number;
  cost_usd: number;
}

function extractEfficiency(obj: Record<string, any>): EfficiencyMetrics {
  let tool_calls = 0;
  let input_tokens = 0;
  let output_tokens = 0;
  let cache_creation_input_tokens = 0;
  let cache_read_input_tokens = 0;
  let cost_usd = 0;

  if (obj.num_turns != null) tool_calls = obj.num_turns;
  if (obj.usage) {
    const u = obj.usage;
    input_tokens = u.input_tokens ?? 0;
    output_tokens = u.output_tokens ?? 0;
    cache_creation_input_tokens = u.cache_creation_input_tokens ?? 0;
    cache_read_input_tokens = u.cache_read_input_tokens ?? 0;
  }
  if (obj.total_cost_usd != null) cost_usd = obj.total_cost_usd;

  const tokens_used = input_tokens + output_tokens + cache_creation_input_tokens + cache_read_input_tokens;
  return { tool_calls, input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens, tokens_used, cost_usd };
}

const ZERO_EFFICIENCY: EfficiencyMetrics = {
  tool_calls: 0, input_tokens: 0, output_tokens: 0,
  cache_creation_input_tokens: 0, cache_read_input_tokens: 0,
  tokens_used: 0, cost_usd: 0,
};

export function scoreEfficiency(agentOutput: string): EfficiencyMetrics {
  // Try stream-json format first (newline-delimited JSON, last object has type=result)
  const resultObj = parseStreamJsonResult(agentOutput);
  if (resultObj) return extractEfficiency(resultObj);

  // Fallback: single JSON object (--output-format json)
  try {
    return extractEfficiency(JSON.parse(agentOutput));
  } catch {
    // Not JSON — degrade gracefully
  }

  return { ...ZERO_EFFICIENCY };
}

/** Parse stream-json output and return the result object (last line with type=result) */
function parseStreamJsonResult(output: string): Record<string, any> | null {
  const lines = output.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (!line) continue;
    try {
      const obj = JSON.parse(line);
      if (obj.type === "result") return obj;
    } catch {
      continue;
    }
  }
  return null;
}

/**
 * Extract all searchable text from agent output.
 * For stream-json: extracts Bash command inputs and tool_result content from all messages.
 * For plain text: returns as-is.
 */
function extractAllText(output: string): string {
  const parts: string[] = [];
  const lines = output.split("\n");
  let isStreamJson = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const obj = JSON.parse(trimmed);
      isStreamJson = true;

      // Extract tool_use inputs (Bash commands, Skill invocations)
      if (obj.type === "assistant") {
        for (const block of obj.message?.content ?? []) {
          if (block.type === "tool_use" && block.input) {
            if (block.name === "Bash" && block.input.command) {
              parts.push(block.input.command);
            }
          }
        }
      }

      // Extract tool_result content (stdout/stderr from Bash)
      if (obj.type === "tool_result") {
        for (const block of obj.content ?? []) {
          if (block.type === "text" && block.text) {
            parts.push(block.text);
          }
        }
      }
    } catch {
      // Not JSON — treat as plain text
      if (!isStreamJson) parts.push(trimmed);
    }
  }

  return parts.join("\n");
}

export function computeTrialScore(
  assertionResults: AssertionResult[],
  negativeResults: { passed: boolean },
  fileSc: number,
  jigUsage: { jig_used: boolean; jig_correct: boolean }
): TrialScore {
  const totalWeight = assertionResults.reduce((sum, a) => sum + a.weight, 0);
  const passedWeight = assertionResults.filter((a) => a.passed).reduce((sum, a) => sum + a.weight, 0);
  const assertion_score = totalWeight > 0 ? passedWeight / totalWeight : 0;
  const negative_score = negativeResults.passed ? 1.0 : 0.0;
  const total = assertion_score * negative_score;

  return {
    assertion_score,
    file_score: fileSc,
    negative_score,
    jig_used: jigUsage.jig_used,
    jig_correct: jigUsage.jig_correct,
    total,
  };
}

// Simple indentation-based scope extraction for Python
function extractScope(content: string, scopeName: string): string {
  const lines = content.split("\n");
  // Strip leading keyword if scope is "class Foo" or "def bar" — the regex already adds the keyword alternation
  const stripped = scopeName.replace(/^(class|def)\s+/, "");
  const pattern = new RegExp(`^(\\s*)(class|def)\\s+${escapeRegex(stripped)}\\b`);

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(pattern);
    if (!match) continue;

    const baseIndent = match[1].length;

    // Include decorators above the def/class (lines starting with @ at same indent)
    const scopeLines: string[] = [];
    for (let d = i - 1; d >= 0; d--) {
      const dl = lines[d];
      if (dl.trim() === "") continue; // skip blank lines between decorators
      const dlIndent = dl.match(/^(\s*)/)?.[1].length ?? 0;
      if (dlIndent === baseIndent && dl.trim().startsWith("@")) {
        scopeLines.unshift(dl);
      } else {
        break;
      }
    }

    scopeLines.push(lines[i]);

    for (let j = i + 1; j < lines.length; j++) {
      const line = lines[j];
      // Empty lines are included
      if (line.trim() === "") {
        scopeLines.push(line);
        continue;
      }
      // Lines with greater indentation are part of the scope
      const lineIndent = line.match(/^(\s*)/)?.[1].length ?? 0;
      if (lineIndent > baseIndent) {
        scopeLines.push(line);
      } else {
        break;
      }
    }
    return scopeLines.join("\n");
  }

  // Scope not found — return empty string so assertion fails explicitly
  return "";
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

