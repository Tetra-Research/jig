# jig MCP Server

MCP (Model Context Protocol) server that wraps jig's CLI as structured tools. Instead of constructing CLI flags, agents call typed MCP tools with JSON parameters and get JSON responses. The server is a thin stdio wrapper — it shells out to the `jig` binary for every operation.

## Prerequisites

- Node.js >= 22
- `jig` binary on PATH ([install instructions](https://github.com/Tetra-Research/jig))

## Quick Start

```bash
npx @jig-cli/mcp-server
```

## Configuration

### Claude Code (`.mcp.json` in project root)

```json
{
  "mcpServers": {
    "jig": {
      "command": "npx",
      "args": ["@jig-cli/mcp-server"]
    }
  }
}
```

### Cursor (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "jig": {
      "command": "npx",
      "args": ["@jig-cli/mcp-server"]
    }
  }
}
```

### Windsurf (`mcp_config.json`)

```json
{
  "mcpServers": {
    "jig": {
      "command": "npx",
      "args": ["@jig-cli/mcp-server"]
    }
  }
}
```

### Codex CLI

```toml
[mcp_servers.jig]
command = "npx"
args = ["@jig-cli/mcp-server"]
```

## Available Tools

| Tool | Description |
|------|-------------|
| `jig_run` | Execute a recipe to create, inject, patch, or replace files from templates |
| `jig_validate` | Validate a recipe or workflow YAML file |
| `jig_vars` | List the variables a recipe or workflow expects |
| `jig_render` | Render a single Jinja2 template with variables |
| `jig_workflow` | Execute a multi-step workflow chaining multiple recipes |

## CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `--jig-path <path>` | Path to jig binary (overrides PATH and JIG_PATH) | — |
| `--timeout <ms>` | Subprocess timeout in milliseconds | 30000 |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `JIG_PATH` | Path to jig binary (overrides PATH lookup) |

## Troubleshooting

**"jig binary not found on PATH"** — Install jig first. See https://github.com/Tetra-Research/jig for instructions.

**"jig command timed out"** — Increase the timeout: `npx @jig-cli/mcp-server --timeout 60000`

**First run is slow** — `npx` downloads the package on first use. For instant starts: `npm install -g @jig-cli/mcp-server`
