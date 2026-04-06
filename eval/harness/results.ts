import fs from "node:fs";
import path from "node:path";
import type { TrialResult } from "./types.ts";

export function writeTrialResult(result: TrialResult, filePath: string): void {
  try {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.appendFileSync(filePath, JSON.stringify(result) + "\n");
  } catch (err) {
    console.error(`[eval] Failed to write trial result to ${filePath}: ${err}`);
  }
}

export function readResults(filePath: string): TrialResult[] {
  if (!fs.existsSync(filePath)) return [];
  const content = fs.readFileSync(filePath, "utf-8").trim();
  if (content === "") return [];
  const results: TrialResult[] = [];
  for (const line of content.split("\n")) {
    try {
      results.push(JSON.parse(line) as TrialResult);
    } catch {
      console.error(`[eval] Skipping malformed JSONL line: ${line.slice(0, 80)}`);
    }
  }
  return results;
}
