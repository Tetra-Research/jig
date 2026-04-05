import fs from "node:fs";
import path from "node:path";
import { normalizeFile } from "./normalize.ts";
import type { Scenario } from "../harness/types.ts";

export function fileScore(actual: string, expected: string): number {
  const normActual = normalizeFile(actual);
  const normExpected = normalizeFile(expected);

  if (normActual === normExpected) return 1.0;

  // Jaccard similarity of trimmed non-empty lines
  const actualLines = new Set(
    normActual.split("\n").map((l) => l.trim()).filter((l) => l.length > 0)
  );
  const expectedLines = new Set(
    normExpected.split("\n").map((l) => l.trim()).filter((l) => l.length > 0)
  );

  let intersection = 0;
  for (const line of actualLines) {
    if (expectedLines.has(line)) intersection++;
  }
  const union = new Set([...actualLines, ...expectedLines]).size;

  if (union === 0) return 1.0; // both empty
  return intersection / union;
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
