import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execSync } from "node:child_process";
import type { ClaudeMdMode, Sandbox, Scenario } from "../harness/types.ts";

export async function createSandbox(
  scenario: Scenario,
  claudeMd: ClaudeMdMode = "shared",
  stripSkills: boolean = false,
): Promise<Sandbox> {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-eval-"));

  // Copy codebase/ contents into the temp dir
  const codebaseDir = path.join(scenario.scenarioDir, "codebase");
  copyDirRecursive(codebaseDir, tmpDir);

  // Strip .claude/skills/ if requested (Level 0 control)
  if (stripSkills) {
    const skillsDir = path.join(tmpDir, ".claude", "skills");
    if (fs.existsSync(skillsDir)) {
      fs.rmSync(skillsDir, { recursive: true, force: true });
    }
    const claudeDir = path.join(tmpDir, ".claude");
    if (fs.existsSync(claudeDir) && fs.readdirSync(claudeDir).length === 0) {
      fs.rmSync(claudeDir, { recursive: true });
    }
  }

  // CLAUDE.md handling based on mode
  const existingClaudeMd = path.join(tmpDir, "CLAUDE.md");
  if (claudeMd === "shared") {
    // Copy shared CLAUDE.md if codebase doesn't already have one
    const sharedDir = path.join(scenario.scenarioDir, "..", "_shared");
    const sharedClaudeMd = path.join(sharedDir, "CLAUDE.md");
    if (fs.existsSync(sharedClaudeMd) && !fs.existsSync(existingClaudeMd)) {
      fs.copyFileSync(sharedClaudeMd, existingClaudeMd);
    }
    // Copy shared skills into .claude/skills/ (e.g., discover skill)
    const sharedSkillsDir = path.join(sharedDir, "skills");
    if (fs.existsSync(sharedSkillsDir)) {
      const targetSkillsDir = path.join(tmpDir, ".claude", "skills");
      fs.mkdirSync(targetSkillsDir, { recursive: true });
      copyDirRecursive(sharedSkillsDir, targetSkillsDir);
    }
  } else if (claudeMd === "empty") {
    // Write an empty CLAUDE.md (overwrite if codebase had one)
    fs.writeFileSync(existingClaudeMd, "# CLAUDE.md\n");
  } else if (claudeMd === "none") {
    // Remove any CLAUDE.md from the codebase
    if (fs.existsSync(existingClaudeMd)) {
      fs.unlinkSync(existingClaudeMd);
    }
  }

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

  const skillsAvailable = !stripSkills && fs.existsSync(path.join(tmpDir, ".claude", "skills"));

  return { workDir: tmpDir, jigVersion, skillsAvailable, cleanup };
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
