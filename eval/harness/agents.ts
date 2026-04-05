import fs from "node:fs";
import { spawn } from "node:child_process";
import { parse as parseYaml } from "yaml";
import type { AgentConfig, AgentResult } from "./types.ts";

export function loadAgentConfigs(filePath: string): AgentConfig[] {
  const raw = fs.readFileSync(filePath, "utf-8");
  const parsed = parseYaml(raw) as { agents: Array<Record<string, unknown>> };

  return parsed.agents.map((a) => ({
    name: a.name as string,
    command: a.command as string,
    args: (a.args as string[]) ?? [],
    timeout_ms: (a.timeout_ms as number) ?? 120000,
    env: a.env as Record<string, string> | undefined,
  }));
}

export function getAgentByName(configs: AgentConfig[], name: string): AgentConfig {
  const agent = configs.find((a) => a.name === name);
  if (!agent) {
    const available = configs.map((a) => a.name).join(", ");
    throw new Error(`Unknown agent "${name}". Available: ${available}`);
  }
  return agent;
}

export function invokeAgent(
  agent: AgentConfig,
  prompt: string,
  workDir: string
): Promise<AgentResult> {
  return new Promise((resolve) => {
    const start = Date.now();
    const args = [...agent.args, prompt];
    const env = { ...process.env, ...(agent.env ?? {}) };

    const child = spawn(agent.command, args, {
      cwd: workDir,
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk: Buffer) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString();
    });

    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGKILL");
    }, agent.timeout_ms);

    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        agent: agent.name,
        exitCode: timedOut ? -1 : (code ?? 1),
        stdout,
        stderr,
        durationMs: Date.now() - start,
        timedOut,
      });
    });

    child.on("error", (err) => {
      clearTimeout(timer);
      resolve({
        agent: agent.name,
        exitCode: -1,
        stdout,
        stderr: stderr + `\nSpawn error: ${err.message}`,
        durationMs: Date.now() - start,
        timedOut: false,
      });
    });
  });
}
