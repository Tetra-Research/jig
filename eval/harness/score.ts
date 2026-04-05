import fs from "node:fs";
import path from "node:path";
import { aggregateFileScore } from "../lib/diff.ts";
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
      const allFiles = readdirRecursive(workDir);
      let found = false;
      for (const f of allFiles) {
        const content = fs.readFileSync(f, "utf-8");
        if (content.includes(na.not_contains)) {
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
        const found = content.includes(na.not_contains);
        results.push({ file: na.file, not_contains: na.not_contains, passed: !found, description: na.description });
      }
    }
  }

  return { passed: results.every((r) => r.passed), results };
}

const JIG_CMD_RE = /\bjig\s+(run|workflow|render)\b/g;
const JIG_VARS_RE = /--vars\s+'([^']+)'/g;

export function scoreJigUsage(
  agentOutput: string,
  scenario: Scenario
): { jig_used: boolean; jig_correct: boolean; call_count: number; invocations: JigInvocation[] } {
  const invocations: JigInvocation[] = [];
  const lines = agentOutput.split("\n");

  for (const line of lines) {
    const cmdMatch = line.match(/\bjig\s+(run|workflow|render)\s+(\S+)/);
    if (!cmdMatch) continue;

    const command = `jig ${cmdMatch[1]} ${cmdMatch[2]}`;
    let vars: string | undefined;
    const varsMatch = line.match(/--vars\s+'([^']+)'/);
    if (varsMatch) vars = varsMatch[1];

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

export function scoreEfficiency(agentOutput: string): {
  tool_calls: number;
  tokens_used: number;
} {
  // Best-effort extraction from Claude Code JSON output
  let tool_calls = 0;
  let tokens_used = 0;

  try {
    // Try to parse as JSON result
    const parsed = JSON.parse(agentOutput);
    if (parsed.num_turns != null) tool_calls = parsed.num_turns;
    if (parsed.usage?.output_tokens != null) {
      tokens_used = (parsed.usage.input_tokens ?? 0) + parsed.usage.output_tokens;
    }
  } catch {
    // Not JSON or not the expected format — degrade gracefully
  }

  return { tool_calls, tokens_used };
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
  const pattern = new RegExp(`^(\\s*)(class|def)\\s+${escapeRegex(scopeName)}\\b`);

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(pattern);
    if (!match) continue;

    const baseIndent = match[1].length;
    const scopeLines = [lines[i]];

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

  // Scope not found — return full content so assertion can still check
  return content;
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function readdirRecursive(dir: string): string[] {
  const results: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === ".git") continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...readdirRecursive(full));
    } else {
      results.push(full);
    }
  }
  return results;
}
