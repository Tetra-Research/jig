import fs from "node:fs";
import path from "node:path";

export function readdirRecursive(dir: string, opts?: { skipGit?: boolean }): string[] {
  const results: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (opts?.skipGit && entry.name === ".git") continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...readdirRecursive(full, opts));
    } else {
      results.push(full);
    }
  }
  return results;
}
