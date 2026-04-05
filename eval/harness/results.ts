import fs from "node:fs";
import type { TrialResult } from "./types.ts";

export function writeTrialResult(result: TrialResult, filePath: string): void {
  try {
    fs.appendFileSync(filePath, JSON.stringify(result) + "\n");
  } catch (err) {
    console.error(`[eval] Failed to write trial result to ${filePath}: ${err}`);
  }
}

export function readResults(filePath: string): TrialResult[] {
  if (!fs.existsSync(filePath)) return [];
  const content = fs.readFileSync(filePath, "utf-8").trim();
  if (content === "") return [];
  return content.split("\n").map((line) => JSON.parse(line) as TrialResult);
}
