# SHARED-CONTEXT.md

> Workstream: mcp-server
> Last updated: 2026-04-04

## Purpose

Build an MCP (Model Context Protocol) stdio server that wraps jig's five CLI commands as typed MCP tools. Agents call structured tools with JSON parameters instead of constructing CLI flags. The server is a thin TypeScript wrapper (~200 lines) that shells out to the `jig` binary — no jig logic is reimplemented.

This is the bridge between jig's CLI interface and the 10+ agentic coding tools that support MCP natively. It turns `jig run`, `jig validate`, `jig vars`, `jig render`, and `jig workflow` into discoverable, typed tools that agents can find via `tools/list` and invoke via `tools/call`.

## Current State

- Initialized (2026-04-04)
- Spec, plan, and narrative written
- No implementation yet — this is a new TypeScript project (`mcp-server/` at repo root)
- All five CLI commands it will wrap are complete and tested (343 tests passing in the Rust codebase)

## Decisions Made

### D-1: TypeScript with @modelcontextprotocol/sdk
The MCP server is TypeScript, not Rust. The `@modelcontextprotocol/sdk` package is the most mature MCP SDK (maintained by Anthropic). The server is ~200 lines — a Rust binary would be overkill. npm distribution (`npx @jig-cli/mcp-server`) matches how most MCP servers are distributed and how every agent's MCP config works.

### D-2: Subprocess wrapper, not library binding
The server shells out to the `jig` binary for every tool call. It does not import jig as a library, does not parse recipes, does not render templates. This keeps it trivially simple and ensures output parity with the CLI. When jig's internals change, the MCP server doesn't need to change — as long as the CLI interface is stable.

### D-3: stdio transport only
All MCP-compatible agents support stdio (launch server as child process, communicate via stdin/stdout). HTTP/SSE transport would add complexity without reaching any additional agents. stdio is sufficient.

### D-4: Five tools matching five CLI commands
One MCP tool per existing CLI command: `jig_run`, `jig_validate`, `jig_vars`, `jig_render`, `jig_workflow`. No stubs for future commands (scan, check, library). Those workstreams add their tools when they're built.

### D-5: vars parameter is a JSON object, not a string
Agents produce JSON objects naturally. The MCP server handles the serialization to a JSON string for the `--vars` CLI flag. This prevents agents from having to think about string escaping inside JSON.

### D-6: --json is always passed to run/workflow
The MCP server hardcodes `--json` for `jig run` and `jig workflow` because it always needs structured output. This is an implementation detail, not exposed as a parameter to the agent.

### D-7: Error format preserves jig's structured error fields
MCP tool errors are `{isError: true, content: text}`. The text content is formatted to include the exit code, jig's stderr (which contains the what/where/why/hint structure), and for exit code 3, the rendered_content from jig's JSON output. This preserves I-4 (structured errors) and I-10 (rendered content for fallback) through the MCP layer.

### D-8: Separate directory, separate package
The MCP server lives in `mcp-server/` at the repo root with its own `package.json` and `tsconfig.json`. It's an npm package, not part of the Rust crate. The jig binary is a runtime dependency found on PATH, not a build dependency.

### D-9: 30-second default subprocess timeout
Recipes and workflows should complete in seconds. 30s is generous but prevents a stuck process from blocking the agent indefinitely. Configurable via `--timeout` CLI flag.

## Patterns Established

(None yet — to be populated during implementation.)

## Known Issues / Tech Debt

### From jig CLI (inherited, not owned by this workstream)
- **C1 from workflows review:** `extract_rendered_from_error` returns error description instead of rendered template content. The MCP server will propagate whatever jig returns — if jig's stderr is wrong, the MCP error content will be wrong too. This is a jig bug to fix in the workflows workstream.
- **M4 from workflows review:** `cmd_run` and `run_recipe` are divergent copies. Doesn't affect the MCP server (it calls the CLI binary, not Rust functions), but indicates the CLI output format could drift between `jig run` and recipe execution within `jig workflow`.

### MCP server specific
- **No Windows support planned.** Binary discovery uses `which` (Unix). jig doesn't currently build for Windows, so this is acceptable. If Windows support is added to jig, the MCP server will need `where` support.
- **npx first-run latency.** First `npx @jig-cli/mcp-server` invocation downloads the package. Document this in README. Suggest `npm install -g` for users who want instant starts.

## File Ownership

| File/Directory | Phase | Description |
|----------------|-------|-------------|
| `mcp-server/` | 1 | **New.** Root directory for the TypeScript MCP server package |
| `mcp-server/package.json` | 1 | **New.** npm package config, dependencies, bin entry, scripts |
| `mcp-server/tsconfig.json` | 1 | **New.** TypeScript compiler config |
| `mcp-server/src/index.ts` | 1 | **New.** MCP server entry point — protocol handling, tools/list, tools/call dispatch |
| `mcp-server/src/tools.ts` | 2 | **New.** Tool definitions (name, description, inputSchema for all five tools) |
| `mcp-server/src/binary.ts` | 2 | **New.** jig binary discovery (PATH, JIG_PATH env, --jig-path flag) and version check |
| `mcp-server/src/invoke.ts` | 3 | **New.** Subprocess invocation — spawn jig, capture stdout/stderr, enforce timeout |
| `mcp-server/src/args.ts` | 3 | **New.** Argument builder functions — translate tool params to CLI flag arrays |
| `mcp-server/src/result.ts` | 3 | **New.** Result translation — JigResult to MCP tool response |
| `mcp-server/test/` | 4 | **New.** Unit and integration tests |
| `mcp-server/README.md` | 5 | **New.** Usage docs with config snippets for Claude Code, Cursor, Windsurf, Codex |
| `.mcp.json` | 5 | **New.** (repo root) MCP config for dogfooding — Claude Code users of this repo get jig tools |

### Files NOT owned by this workstream
- `src/*.rs` — all Rust source. The MCP server does not modify jig's internals.
- `tests/fixtures/` — existing test fixtures. The MCP server reads them in integration tests but doesn't modify them.
- `Cargo.toml`, `Cargo.lock` — Rust package config. Untouched.
- `CLAUDE.md` — updated only to note MCP server existence in project status (Phase 5).
