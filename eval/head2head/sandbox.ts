import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execSync } from "node:child_process";
import type { Scenario } from "../harness/types.ts";
import type { HeadToHeadArmConfig, HeadToHeadSandbox } from "./types.ts";

interface CreateHeadToHeadSandboxOptions {
  cleanSlate: boolean;
}

export async function createHeadToHeadSandbox(
  scenario: Scenario,
  arm: HeadToHeadArmConfig,
  options: CreateHeadToHeadSandboxOptions
): Promise<HeadToHeadSandbox> {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-head2head-"));

  const codebaseDir = path.join(scenario.scenarioDir, "codebase");
  copyDirRecursive(codebaseDir, tmpDir);

  if (options.cleanSlate) {
    resetClaudeState(tmpDir);
  }

  const profilePath = path.resolve(arm.profilePath);
  if (!fs.existsSync(profilePath) || !fs.statSync(profilePath).isDirectory()) {
    throw new Error(`Profile directory not found: ${profilePath}`);
  }

  applyProfile(profilePath, tmpDir);

  // Ensure deterministic diff state.
  execSync("git init", { cwd: tmpDir, stdio: "pipe" });
  execSync("git add -A", { cwd: tmpDir, stdio: "pipe" });
  execSync('git commit -m "initial" --allow-empty', {
    cwd: tmpDir,
    stdio: "pipe",
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: "jig-head2head",
      GIT_AUTHOR_EMAIL: "head2head@jig",
      GIT_COMMITTER_NAME: "jig-head2head",
      GIT_COMMITTER_EMAIL: "head2head@jig",
    },
  });

  const installedSkills = listInstalledSkills(tmpDir);
  const hasClaudeMd = fs.existsSync(path.join(tmpDir, "CLAUDE.md"));
  const jigVersion = getJigVersion(tmpDir);

  const cleanup = async () => {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // best effort cleanup
    }
  };

  return {
    workDir: tmpDir,
    jigVersion,
    profilePath,
    installedSkills,
    hasClaudeMd,
    cleanup,
  };
}

function resetClaudeState(workDir: string): void {
  const claudeDir = path.join(workDir, ".claude");
  if (fs.existsSync(claudeDir)) {
    fs.rmSync(claudeDir, { recursive: true, force: true });
  }

  const claudeMd = path.join(workDir, "CLAUDE.md");
  if (fs.existsSync(claudeMd)) {
    fs.rmSync(claudeMd, { force: true });
  }
}

function applyProfile(profilePath: string, workDir: string): void {
  const overlayDir = path.join(profilePath, "overlay");
  if (fs.existsSync(overlayDir) && fs.statSync(overlayDir).isDirectory()) {
    copyDirRecursive(overlayDir, workDir);
    return;
  }

  const profileClaudeMd = path.join(profilePath, "CLAUDE.md");
  if (fs.existsSync(profileClaudeMd)) {
    fs.copyFileSync(profileClaudeMd, path.join(workDir, "CLAUDE.md"));
  }

  const directSkillsDir = path.join(profilePath, "skills");
  const nestedSkillsDir = path.join(profilePath, ".claude", "skills");
  let sourceSkillsDir: string | undefined;
  if (fs.existsSync(directSkillsDir) && fs.statSync(directSkillsDir).isDirectory()) {
    sourceSkillsDir = directSkillsDir;
  } else if (fs.existsSync(nestedSkillsDir) && fs.statSync(nestedSkillsDir).isDirectory()) {
    sourceSkillsDir = nestedSkillsDir;
  }

  if (sourceSkillsDir) {
    const targetSkillsDir = path.join(workDir, ".claude", "skills");
    fs.mkdirSync(targetSkillsDir, { recursive: true });
    copyDirRecursive(sourceSkillsDir, targetSkillsDir);
  }
}

function listInstalledSkills(workDir: string): string[] {
  const skillsDir = path.join(workDir, ".claude", "skills");
  if (!fs.existsSync(skillsDir) || !fs.statSync(skillsDir).isDirectory()) {
    return [];
  }

  return fs
    .readdirSync(skillsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
}

function getJigVersion(cwd: string): string {
  const candidates = ["jig"];
  const projectRoot = path.resolve(import.meta.dirname ?? ".", "../..");
  candidates.push(path.join(projectRoot, "target", "release", "jig"));
  candidates.push(path.join(projectRoot, "target", "debug", "jig"));

  for (const candidate of candidates) {
    try {
      return execSync(`${candidate} --version`, { cwd, stdio: "pipe", encoding: "utf-8" }).trim();
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
      continue;
    }

    fs.mkdirSync(path.dirname(destPath), { recursive: true });
    fs.copyFileSync(srcPath, destPath);
  }
}
