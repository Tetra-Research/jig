#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerTools } from "./tools.js";
import { findJigBinary, getJigVersion } from "./binary.js";
import { invokeJig } from "./invoke.js";
import {
  buildRunArgs,
  buildValidateArgs,
  buildVarsArgs,
  buildRenderArgs,
  buildWorkflowArgs,
} from "./args.js";
import { translateResult } from "./result.js";

function parseArgs(argv: string[]): { jigPath?: string; timeout: number } {
  let jigPath: string | undefined;
  let timeout = 30000;
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--jig-path" && argv[i + 1]) {
      jigPath = argv[++i];
    } else if (argv[i] === "--timeout" && argv[i + 1]) {
      const parsed = parseInt(argv[++i], 10);
      if (!Number.isNaN(parsed) && parsed > 0) {
        timeout = parsed;
      }
    }
  }
  return { jigPath, timeout };
}

async function main() {
  const { jigPath, timeout } = parseArgs(process.argv);

  const jigBinaryPath = findJigBinary(jigPath);
  if (jigBinaryPath) {
    const version = getJigVersion(jigBinaryPath);
    if (version) {
      console.error(`jig MCP server: found jig ${version} at ${jigBinaryPath}`);
    } else {
      console.error(`jig MCP server: warning: jig at ${jigBinaryPath} did not respond to --version`);
    }
  } else {
    if (jigPath) {
      console.error(`jig MCP server: warning: --jig-path ${jigPath} is not executable or does not exist`);
    } else if (process.env["JIG_PATH"]) {
      console.error(`jig MCP server: warning: JIG_PATH=${process.env["JIG_PATH"]} is not executable or does not exist`);
    } else {
      console.error("jig MCP server: warning: jig binary not found on PATH");
    }
  }

  const server = new McpServer({ name: "jig", version: "0.1.0" });

  const cwd = process.cwd();

  registerTools(server, async (toolName, params) => {
    if (!jigBinaryPath) {
      return {
        content: [
          {
            type: "text" as const,
            text: "jig binary not found on PATH. Install jig first: see https://github.com/Tetra-Research/jig",
          },
        ],
        isError: true,
      };
    }

    let args: string[];
    switch (toolName) {
      case "jig_run":
        args = buildRunArgs(params as { recipe: string; vars?: object; dry_run?: boolean; base_dir?: string; force?: boolean; verbose?: boolean });
        break;
      case "jig_validate":
        args = buildValidateArgs(params as { path: string });
        break;
      case "jig_vars":
        args = buildVarsArgs(params as { path: string });
        break;
      case "jig_render":
        args = buildRenderArgs(params as { template: string; vars?: object; to?: string });
        break;
      case "jig_workflow":
        args = buildWorkflowArgs(params as { workflow: string; vars?: object; dry_run?: boolean; base_dir?: string; force?: boolean; verbose?: boolean });
        break;
      default:
        return {
          content: [{ type: "text" as const, text: `Unknown tool: ${toolName}` }],
          isError: true,
        };
    }

    const result = await invokeJig(jigBinaryPath, args, cwd, timeout);
    return translateResult(toolName, result, params as Record<string, unknown>);
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("jig MCP server fatal error:", err);
  process.exit(1);
});
