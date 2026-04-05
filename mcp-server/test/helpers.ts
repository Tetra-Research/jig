import { spawn, type ChildProcess } from "node:child_process";
import { join } from "node:path";

const SERVER_PATH = join(import.meta.dirname, "..", "dist", "index.js");

export interface McpClient {
  proc: ChildProcess;
  send(method: string, params?: unknown, id?: number): void;
  readResponse(): Promise<Record<string, unknown>>;
  readNotification(): Promise<Record<string, unknown>>;
  close(): Promise<number>;
  initialize(): Promise<Record<string, unknown>>;
  callTool(name: string, args?: Record<string, unknown>): Promise<Record<string, unknown>>;
  listTools(): Promise<Record<string, unknown>>;
}

let nextId = 1;

export function createClient(extraArgs: string[] = [], env?: Record<string, string>): McpClient {
  const proc = spawn("node", [SERVER_PATH, ...extraArgs], {
    stdio: ["pipe", "pipe", "pipe"],
    cwd: join(import.meta.dirname, "..", ".."),
    env: { ...process.env, ...env },
  });

  let closed = false;
  let exitCode: number | null = null;

  proc.on("exit", (code) => {
    closed = true;
    exitCode = code ?? 0;
  });

  let buffer = "";
  const responses: Record<string, unknown>[] = [];
  const notifications: Record<string, unknown>[] = [];
  const waiters: Array<(msg: Record<string, unknown>) => void> = [];
  const notificationWaiters: Array<(msg: Record<string, unknown>) => void> = [];

  proc.stdout!.setEncoding("utf-8");
  proc.stdout!.on("data", (chunk: string) => {
    buffer += chunk;
    const lines = buffer.split("\n");
    buffer = lines.pop()!;
    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        const msg = JSON.parse(line) as Record<string, unknown>;
        if ("id" in msg) {
          if (waiters.length > 0) {
            waiters.shift()!(msg);
          } else {
            responses.push(msg);
          }
        } else {
          if (notificationWaiters.length > 0) {
            notificationWaiters.shift()!(msg);
          } else {
            notifications.push(msg);
          }
        }
      } catch {
        // ignore non-JSON lines
      }
    }
  });

  function send(method: string, params?: unknown, id?: number) {
    const msg: Record<string, unknown> = { jsonrpc: "2.0", method };
    if (params !== undefined) msg.params = params;
    if (id !== undefined) msg.id = id;
    proc.stdin!.write(JSON.stringify(msg) + "\n");
  }

  function readResponse(): Promise<Record<string, unknown>> {
    if (responses.length > 0) {
      return Promise.resolve(responses.shift()!);
    }
    return new Promise((resolve) => {
      waiters.push(resolve);
    });
  }

  function readNotification(): Promise<Record<string, unknown>> {
    if (notifications.length > 0) {
      return Promise.resolve(notifications.shift()!);
    }
    return new Promise((resolve) => {
      notificationWaiters.push(resolve);
    });
  }

  async function close(): Promise<number> {
    if (closed) return exitCode ?? 0;
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        proc.kill("SIGKILL");
      }, 3000);
      proc.on("exit", (code) => {
        clearTimeout(timer);
        resolve(code ?? 0);
      });
      proc.stdin!.end();
    });
  }

  async function initialize(): Promise<Record<string, unknown>> {
    send("initialize", {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "test-client", version: "1.0.0" },
    }, nextId++);
    const result = await readResponse();
    // send initialized notification
    send("notifications/initialized");
    return result;
  }

  async function callTool(name: string, args?: Record<string, unknown>): Promise<Record<string, unknown>> {
    send("tools/call", { name, arguments: args ?? {} }, nextId++);
    return readResponse();
  }

  async function listTools(): Promise<Record<string, unknown>> {
    send("tools/list", {}, nextId++);
    return readResponse();
  }

  return { proc, send, readResponse, readNotification, close, initialize, callTool, listTools };
}
