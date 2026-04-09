import fs from "node:fs";
import path from "node:path";
import { normalizeFile } from "./normalize.ts";
import type { Scenario } from "../harness/types.ts";

export function fileScore(actual: string, expected: string): number {
  const actualLines = comparableLines(actual);
  const expectedLines = comparableLines(expected);

  if (actualLines.length === expectedLines.length && actualLines.every((line, index) => line === expectedLines[index])) {
    return 1.0;
  }

  const maxLen = Math.max(actualLines.length, expectedLines.length);
  if (maxLen === 0) return 1.0;

  return longestCommonSubsequenceLength(actualLines, expectedLines) / maxLen;
}

export function aggregateFileScore(scenario: Scenario, workDir: string): number {
  if (scenario.expected_files_modified.length === 0) return 1.0;

  const expectedDir = path.join(scenario.scenarioDir, "expected");
  let total = 0;

  for (const file of scenario.expected_files_modified) {
    const expectedPath = path.join(expectedDir, file);
    const actualPath = path.join(workDir, file);

    if (!fs.existsSync(actualPath)) {
      total += 0.0; // missing file = 0
      continue;
    }

    const expectedContent = fs.readFileSync(expectedPath, "utf-8");
    const actualContent = fs.readFileSync(actualPath, "utf-8");
    total += fileScore(actualContent, expectedContent);
  }

  return total / scenario.expected_files_modified.length;
}

function comparableLines(content: string): string[] {
  return normalizeFile(content)
    .split("\n")
    .filter((line) => line.trim().length > 0);
}

function longestCommonSubsequenceLength(actual: string[], expected: string[]): number {
  const previous = new Array(expected.length + 1).fill(0);
  const current = new Array(expected.length + 1).fill(0);

  for (let i = 1; i <= actual.length; i++) {
    current[0] = 0;
    for (let j = 1; j <= expected.length; j++) {
      if (actual[i - 1] === expected[j - 1]) {
        current[j] = previous[j - 1] + 1;
      } else {
        current[j] = Math.max(previous[j], current[j - 1]);
      }
    }

    for (let j = 0; j <= expected.length; j++) {
      previous[j] = current[j];
    }
  }

  return previous[expected.length];
}
