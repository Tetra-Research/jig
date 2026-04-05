import { execFileSync } from "node:child_process";
import { accessSync, constants } from "node:fs";

function isExecutable(path: string): boolean {
  try {
    accessSync(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export function findJigBinary(cliPath?: string): string | null {
  if (cliPath) return isExecutable(cliPath) ? cliPath : null;

  const envPath = process.env["JIG_PATH"];
  if (envPath) return isExecutable(envPath) ? envPath : null;

  try {
    const result = execFileSync("which", ["jig"], { encoding: "utf-8" });
    return result.trim() || null;
  } catch {
    return null;
  }
}

export function getJigVersion(binaryPath: string): string | null {
  try {
    const result = execFileSync(binaryPath, ["--version"], {
      encoding: "utf-8",
      timeout: 5000,
    });
    const match = result.match(/\d+\.\d+\.\d+/);
    return match ? match[0] : null;
  } catch {
    return null;
  }
}
