import type { JigResult } from "./invoke.js";

const EXIT_CODE_MEANINGS: Record<number, string> = {
  1: "recipe validation error",
  2: "template rendering error",
  3: "file operation error",
  4: "variable validation error",
};

export interface McpToolResponse {
  content: Array<{ type: "text"; text: string }>;
  isError?: boolean;
}

export function translateResult(
  toolName: string,
  result: JigResult,
  params?: Record<string, unknown>
): McpToolResponse {
  // Success
  if (result.exitCode === 0) {
    // AC-4.7: jig_render with --to returns confirmation, not empty string
    if (toolName === "jig_render" && params?.to && !result.stdout.trim()) {
      return {
        content: [{ type: "text", text: `Rendered template to ${params.to}` }],
        isError: false,
      };
    }
    return {
      content: [{ type: "text", text: result.stdout }],
      isError: false,
    };
  }

  // Timeout
  if (result.exitCode === -2) {
    return {
      content: [{ type: "text", text: result.stderr }],
      isError: true,
    };
  }

  // Spawn failure (ENOENT, EACCES)
  if (result.exitCode === -1) {
    if (result.stderr.includes("ENOENT")) {
      return {
        content: [
          {
            type: "text",
            text: "jig binary not found on PATH. Install jig first: see https://github.com/Tetra-Research/jig",
          },
        ],
        isError: true,
      };
    }
    return {
      content: [{ type: "text", text: result.stderr }],
      isError: true,
    };
  }

  // jig error (exit codes 1-4)
  const meaning = EXIT_CODE_MEANINGS[result.exitCode] ?? "unknown error";
  let text = `jig exited with code ${result.exitCode} (${meaning})\n\n${result.stderr}`;

  // AC-4.2: Include stdout if non-empty (structured errors may be in stdout JSON)
  if (result.stdout.trim()) {
    // For exit code 3, try to extract rendered_content for LLM fallback (AC-4.3)
    if (result.exitCode === 3) {
      try {
        const json = JSON.parse(result.stdout);
        const rendered = extractRenderedContent(json);
        if (rendered) {
          text += `\n\nRendered content (for manual fallback):\n${rendered}`;
        }
      } catch {
        // stdout wasn't valid JSON; include raw
        text += `\n\n${result.stdout}`;
      }
    } else {
      text += `\n\n${result.stdout}`;
    }
  }

  return {
    content: [{ type: "text", text }],
    isError: true,
  };
}

function extractRenderedContent(json: unknown): string | null {
  if (!json || typeof json !== "object") return null;

  const obj = json as Record<string, unknown>;

  // Check direct rendered_content field
  if (typeof obj["rendered_content"] === "string") {
    return obj["rendered_content"];
  }

  // Check operations array for rendered_content
  if (Array.isArray(obj["operations"])) {
    const contents: string[] = [];
    for (const op of obj["operations"]) {
      if (op && typeof op === "object" && typeof op["rendered_content"] === "string") {
        contents.push(op["rendered_content"]);
      }
    }
    if (contents.length > 0) return contents.join("\n---\n");
  }

  return null;
}
