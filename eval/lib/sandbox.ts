import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execSync } from "node:child_process";
import type { Sandbox, Scenario } from "../harness/types.ts";

export async function createSandbox(scenario: Scenario): Promise<Sandbox> {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-eval-"));

  // Copy codebase/ contents into the temp dir
  const codebaseDir = path.join(scenario.scenarioDir, "codebase");
  copyDirRecursive(codebaseDir, tmpDir);

  // Git init + initial commit
  execSync("git init", { cwd: tmpDir, stdio: "pipe" });
  execSync("git add -A", { cwd: tmpDir, stdio: "pipe" });
  execSync('git commit -m "initial" --allow-empty', { cwd: tmpDir, stdio: "pipe", env: { ...process.env, GIT_AUTHOR_NAME: "jig-eval", GIT_AUTHOR_EMAIL: "eval@jig", GIT_COMMITTER_NAME: "jig-eval", GIT_COMMITTER_EMAIL: "eval@jig" } });

  // Verify jig is available and capture version
  const jigVersion = getJigVersion(tmpDir);

  const cleanup = async () => {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // best-effort cleanup
    }
  };

  return { workDir: tmpDir, jigVersion, cleanup };
}

function getJigVersion(cwd: string): string {
  // Try which jig first, then fallback paths
  const candidates = ["jig"];
  const projectRoot = path.resolve(import.meta.dirname ?? ".", "../..");
  candidates.push(path.join(projectRoot, "target", "release", "jig"));
  candidates.push(path.join(projectRoot, "target", "debug", "jig"));

  for (const candidate of candidates) {
    try {
      const version = execSync(`${candidate} --version`, { cwd, stdio: "pipe", encoding: "utf-8" }).trim();
      return version;
    } catch {
      continue;
    }
  }
  return "unknown";
}

function copyDirRecursive(src: string, dest: string): void {
  if (!fs.existsSync(src)) return;
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      fs.mkdirSync(destPath, { recursive: true });
      copyDirRecursive(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}
