import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

export interface WorkspaceArtifactPaths {
  git_status?: string;
  diff_stat?: string;
  diff_patch?: string;
  changed_files_manifest?: string;
  workspace_snapshot_dir?: string;
}

export function writeWorkspaceArtifacts(artifactDir: string, workDir: string): WorkspaceArtifactPaths {
  const out: WorkspaceArtifactPaths = {};
  fs.mkdirSync(artifactDir, { recursive: true });

  try {
    execGit(workDir, ["add", "-N", "."]);
  } catch {
    // best effort
  }

  const statusText = execGit(workDir, ["status", "--short", "--untracked-files=all"]);
  if (statusText != null) {
    const statusPath = path.join(artifactDir, "git-status.txt");
    fs.writeFileSync(statusPath, statusText, "utf-8");
    out.git_status = statusPath;
  }

  const diffStatText = execGit(workDir, ["diff", "--stat", "--find-renames", "HEAD"]);
  if (diffStatText != null) {
    const diffStatPath = path.join(artifactDir, "git-diff-stat.txt");
    fs.writeFileSync(diffStatPath, diffStatText, "utf-8");
    out.diff_stat = diffStatPath;
  }

  const diffPatchText = execGit(workDir, ["diff", "--find-renames", "--binary", "HEAD"]);
  if (diffPatchText != null) {
    const diffPatchPath = path.join(artifactDir, "git-diff.patch");
    fs.writeFileSync(diffPatchPath, diffPatchText, "utf-8");
    out.diff_patch = diffPatchPath;
  }

  const changedFiles = listChangedFiles(workDir);
  const manifestPath = path.join(artifactDir, "changed-files.txt");
  fs.writeFileSync(manifestPath, changedFiles.join("\n") + (changedFiles.length > 0 ? "\n" : ""), "utf-8");
  out.changed_files_manifest = manifestPath;

  if (changedFiles.length > 0) {
    const snapshotDir = path.join(artifactDir, "workspace");
    fs.mkdirSync(snapshotDir, { recursive: true });
    for (const relativePath of changedFiles) {
      const srcPath = path.join(workDir, relativePath);
      if (!fs.existsSync(srcPath) || !fs.statSync(srcPath).isFile()) {
        continue;
      }
      const destPath = path.join(snapshotDir, relativePath);
      fs.mkdirSync(path.dirname(destPath), { recursive: true });
      fs.copyFileSync(srcPath, destPath);
    }
    out.workspace_snapshot_dir = snapshotDir;
  }

  return out;
}

function listChangedFiles(workDir: string): string[] {
  const tracked = splitLines(execGit(workDir, ["diff", "--name-only", "HEAD"]));
  const untracked = splitLines(execGit(workDir, ["ls-files", "--others", "--exclude-standard"]));
  return [...new Set([...tracked, ...untracked])].sort();
}

function splitLines(value: string | undefined): string[] {
  if (!value) return [];
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function execGit(workDir: string, args: string[]): string | undefined {
  try {
    return execFileSync("git", args, {
      cwd: workDir,
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch {
    return undefined;
  }
}
