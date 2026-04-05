import { execFile } from "node:child_process";

export interface JigResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

export function invokeJig(
  binaryPath: string,
  args: string[],
  cwd: string,
  timeout: number
): Promise<JigResult> {
  return new Promise((resolve) => {
    execFile(
      binaryPath,
      args,
      { cwd, timeout, maxBuffer: 10 * 1024 * 1024, encoding: "utf-8" },
      (error, stdout, stderr) => {
        if (error) {
          if ("killed" in error && error.killed) {
            resolve({
              exitCode: -2,
              stdout: stdout ?? "",
              stderr: `jig command timed out after ${Math.round(timeout / 1000)} seconds`,
            });
            return;
          }
          if (error.code === "ENOENT" || error.code === "EACCES") {
            resolve({
              exitCode: -1,
              stdout: "",
              stderr: error.message,
            });
            return;
          }
          resolve({
            exitCode: (error as NodeJS.ErrnoException & { status?: number }).status ?? 1,
            stdout: stdout ?? "",
            stderr: stderr ?? "",
          });
          return;
        }
        resolve({ exitCode: 0, stdout: stdout ?? "", stderr: stderr ?? "" });
      }
    );
  });
}
