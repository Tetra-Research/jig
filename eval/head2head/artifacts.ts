import fs from "node:fs";
import path from "node:path";
import type { AgentArtifactPaths } from "../harness/types.ts";
import type { HeadToHeadArm } from "./types.ts";
import { writeWorkspaceArtifacts } from "../lib/workspace-artifacts.ts";

interface WriteHeadToHeadArtifactsInput {
  artifactsRoot: string;
  scenario: string;
  agent: string;
  arm: HeadToHeadArm;
  rep: number;
  prompt: string;
  stdout: string;
  stderr: string;
  workDir: string;
}

function slug(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 80);
}

function makeRunId(input: WriteHeadToHeadArtifactsInput): string {
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const rand = Math.random().toString(36).slice(2, 8);
  return [
    ts,
    slug(input.scenario),
    slug(input.agent),
    input.arm,
    `rep${input.rep}`,
    rand,
  ].join("__");
}

export function writeHeadToHeadArtifacts(input: WriteHeadToHeadArtifactsInput): AgentArtifactPaths {
  const runId = makeRunId(input);
  const dir = path.resolve(input.artifactsRoot, runId);
  fs.mkdirSync(dir, { recursive: true });

  const promptPath = path.join(dir, "prompt.txt");
  const stdoutPath = path.join(dir, "stdout.log");
  const stderrPath = path.join(dir, "stderr.log");
  const combinedPath = path.join(dir, "combined.log");

  fs.writeFileSync(promptPath, input.prompt, "utf-8");
  fs.writeFileSync(stdoutPath, input.stdout, "utf-8");
  fs.writeFileSync(stderrPath, input.stderr, "utf-8");
  fs.writeFileSync(combinedPath, `${input.stdout}\n${input.stderr}`, "utf-8");

  return {
    dir,
    prompt: promptPath,
    stdout: stdoutPath,
    stderr: stderrPath,
    combined: combinedPath,
    ...writeWorkspaceArtifacts(dir, input.workDir),
  };
}
