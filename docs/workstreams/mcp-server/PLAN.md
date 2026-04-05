# PLAN.md

> Workstream: mcp-server
> Last updated: 2026-04-04
> Status: Planned

## Objective

Build an MCP (Model Context Protocol) server that wraps jig's CLI as structured tools for agentic coding tools. The server is a thin stdio wrapper — TypeScript, ~200 lines — that translates MCP tool calls into jig subprocess invocations and returns structured results.

The deliverable is: an npm package (`@jig-cli/mcp-server`) that can be configured in Claude Code, Codex, Cursor, Windsurf, or any MCP-compatible agent with a one-line config entry. Agents get typed tool definitions for `jig_run`, `jig_validate`, `jig_vars`, `jig_render`, and `jig_workflow` — no CLI flag guessing, no `--help` parsing.

## Phases

### Phase 1: Project Scaffold & MCP Protocol
Status: Planned

Set up the TypeScript project with the MCP SDK and implement the core protocol handling: initialize handshake, tools/list, and the JSON-RPC message loop.

#### Milestones
- [ ] 1.1: Create `mcp-server/` directory at repo root with `package.json` (name: `@jig-cli/mcp-server`), `tsconfig.json`, and TypeScript tooling (`tsx` for dev, `tsup` or `tsc` for build)
- [ ] 1.2: Add `@modelcontextprotocol/sdk` as primary dependency. Add `typescript`, `@types/node` as dev dependencies
- [ ] 1.3: Implement `src/index.ts` — MCP server entry point using `@modelcontextprotocol/sdk`'s `Server` class with stdio transport. Handle `initialize` (return server info + tool capability), `initialized` notification, and `tools/list` (return empty tools array initially)
- [ ] 1.4: Add `bin` entry in `package.json` pointing to the compiled entry point. Verify `npx .` launches the server and responds to initialize
- [ ] 1.5: Add basic protocol tests: initialize handshake, tools/list response shape, malformed JSON-RPC handling, clean EOF shutdown

#### Validation Criteria
- Server starts, completes MCP initialize handshake, and responds to tools/list
- Malformed JSON-RPC returns appropriate error codes (-32700, -32600)
- stdin EOF causes clean shutdown (exit 0)
- `npx .` from the mcp-server directory starts the server

### Phase 2: Tool Definitions & Binary Discovery
Status: Planned

Define the five MCP tools with complete JSON Schema input definitions and implement jig binary discovery.

#### Milestones
- [ ] 2.1: Create `src/tools.ts` — export the five tool definitions (jig_run, jig_validate, jig_vars, jig_render, jig_workflow) with name, description, and inputSchema. Descriptions must be clear enough for an LLM to select the right tool without prior jig knowledge
- [ ] 2.2: Wire tool definitions into `tools/list` handler — tools/list now returns all five tools
- [ ] 2.3: Create `src/binary.ts` — implement `findJigBinary()`: check `--jig-path` CLI flag, then `JIG_PATH` env var, then search PATH using `which`/`where`. Implement `getJigVersion()`: run `jig --version`, capture version string
- [ ] 2.4: On server startup, locate jig binary and log path + version to stderr. If not found, log warning to stderr but continue (tool calls will fail with helpful error)
- [ ] 2.5: Add tests: tool definitions match SPEC.md schemas, binary discovery with JIG_PATH override, binary not found handling

#### Validation Criteria
- `tools/list` returns exactly five tools with complete inputSchema
- Each tool description is self-contained (an agent unfamiliar with jig can understand what the tool does)
- Binary discovery works via PATH, JIG_PATH env, and --jig-path flag
- Missing binary logs warning but doesn't crash

### Phase 3: CLI Invocation & Result Translation
Status: Planned

Implement the core dispatch logic: translate tool calls into CLI invocations, run jig as a subprocess, and translate results into MCP responses.

#### Milestones
- [ ] 3.1: Create `src/invoke.ts` — implement `invokeJig(binaryPath, args, cwd, timeout)`: spawn jig subprocess, capture stdout/stderr, enforce timeout, return `JigResult { exitCode, stdout, stderr }`
- [ ] 3.2: Create `src/args.ts` — implement argument builder functions for each tool: `buildRunArgs(params)`, `buildValidateArgs(params)`, `buildVarsArgs(params)`, `buildRenderArgs(params)`, `buildWorkflowArgs(params)`. Each translates typed params to CLI flag arrays. Key behaviors: serialize `vars` object to JSON string for `--vars` flag; always pass `--json` for run/workflow; omit `--vars` when vars is undefined/empty
- [ ] 3.3: Create `src/result.ts` — implement `translateResult(toolName, result)`: exit code 0 → `{isError: false, content: stdout}`, exit codes 1-4 → `{isError: true, content: formatted error}`. For exit code 3, extract `rendered_content` from jig's JSON output and append to error content
- [ ] 3.4: Wire tools/call handler in `src/index.ts` — dispatch by tool name, build args, invoke jig, translate result, return MCP response. Unknown tool name returns JSON-RPC error -32601
- [ ] 3.5: Add unit tests for argument builders: verify correct flag generation for all parameter combinations (with vars, without vars, with dry_run, with base_dir, etc.)
- [ ] 3.6: Add unit tests for result translator: success case, each exit code (1-4), binary not found, timeout, rendered_content extraction for code 3

#### Validation Criteria
- `jig_run` tool call with a valid recipe produces the same JSON as `jig run --json`
- `vars` object parameter is correctly serialized as JSON string for `--vars`
- Omitted `vars` results in no `--vars` flag (not `--vars '{}'`)
- Error responses include exit code, structured error from stderr, and rendered_content for code 3
- Subprocess timeout kills process and returns error
- Binary not found returns helpful error message

### Phase 4: Integration Testing with Fixtures
Status: Planned

End-to-end tests that start the MCP server, send JSON-RPC messages, and verify responses against known jig outputs using the existing test fixtures.

#### Milestones
- [ ] 4.1: Create test harness: `test/e2e.test.ts` — spawns the MCP server as a child process, sends JSON-RPC messages to its stdin, reads responses from stdout, and asserts on response content
- [ ] 4.2: Test `jig_run` end-to-end: use a simple create-only fixture from `tests/fixtures/`, invoke via MCP, verify the response JSON matches `jig run --json` output
- [ ] 4.3: Test `jig_validate` end-to-end: validate a recipe fixture, verify validation output
- [ ] 4.4: Test `jig_vars` end-to-end: get vars from a recipe fixture, verify JSON matches
- [ ] 4.5: Test `jig_render` end-to-end: render a template from a fixture, verify rendered content
- [ ] 4.6: Test `jig_workflow` end-to-end: run a workflow fixture, verify per-step results
- [ ] 4.7: Test error paths: missing recipe, bad variables, failing injection (verify rendered_content in error)
- [ ] 4.8: Test determinism: run the same tool call twice, verify byte-identical responses

#### Validation Criteria
- All five tools produce correct results end-to-end
- Error responses preserve jig's structured error information
- Rendered content is included in file operation errors
- Results are deterministic across runs

### Phase 5: Packaging, Documentation & Cross-Tool Verification
Status: Planned

Package for npm distribution, write configuration documentation, and verify the server works with Claude Code.

#### Milestones
- [ ] 5.1: Configure build: `tsup` or `tsc` to compile TypeScript to a single JS entry point. Add `bin` field, `files` field, and `prepublishOnly` script to `package.json`
- [ ] 5.2: Add `#!/usr/bin/env node` shebang to built output for direct execution
- [ ] 5.3: Write `mcp-server/README.md` with: what it does (one paragraph), prerequisites (jig on PATH), configuration snippets for Claude Code `.mcp.json`, Cursor `.cursor/mcp.json`, Windsurf `mcp_config.json`, and Codex CLI `config.toml`
- [ ] 5.4: Add `.mcp.json` to the jig repo root for dogfooding — configures the MCP server for Claude Code users of this repo
- [ ] 5.5: Smoke test with Claude Code: configure in `.mcp.json`, start a Claude Code session, verify tools appear, invoke `jig_vars` and `jig_run` on a test recipe
- [ ] 5.6: Run full test suite (`npm test`), verify all pass, update CLAUDE.md project status

#### Validation Criteria
- `npx @jig-cli/mcp-server` starts the server (local testing with `npx .`)
- Claude Code sees all five tools after configuration
- A real tool call from Claude Code produces correct results
- README has working config snippets for 4+ agentic tools
- All tests pass

## Dependencies

- **Depends on:** core-engine (v0.1, complete), replace-patch (v0.2, complete), workflows (v0.3, complete). The MCP server wraps all five CLI commands, which exercise all four operation types and workflow orchestration.
- **Blocks:** Nothing directly. But the MCP server is additive for all future workstreams — `scan` (v0.7), `check` (v0.7), and `library` (v0.4) will add their tools to the MCP server when they're built.
- **External dependency:** `@modelcontextprotocol/sdk` npm package (mature, maintained by Anthropic). No other non-dev external dependencies.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Implementation language | TypeScript | The MCP SDK for TS (`@modelcontextprotocol/sdk`) is the most mature. The server is ~200 lines — doesn't justify a Rust binary for this. npm distribution (`npx @jig-cli/mcp-server`) is how most MCP servers are distributed. Every agent's MCP config supports `npx` commands. |
| Architecture | Subprocess wrapper (shell out to jig binary) | The spec mandates this: "does not reimplement jig's logic — it shells out to the jig binary." This keeps the server trivially simple, ensures output parity with CLI, and means the server doesn't need to track jig internal changes. |
| Transport | stdio only | All MCP-compatible agents support stdio. HTTP/SSE adds complexity for no gain — agents launch the server as a child process. |
| Which tools to expose | 5 tools matching the 5 existing CLI commands | Expose what exists today. Don't create stubs for future commands (scan, check, library). Those workstreams will add their tools when they're built. |
| vars parameter type | `object` (not `string`) | Agents produce JSON objects naturally. The server serializes to string for `--vars`. This prevents agents from having to JSON-encode-then-string-escape variables. |
| --json always for run/workflow | Yes — hardcoded | The MCP server always needs structured output from jig. Don't expose this as a parameter — it's an implementation detail of the translation layer. |
| Error format | Structured text with exit code + stderr + rendered_content | MCP tool errors are text content with `isError: true`. We format it to preserve jig's structured error fields. The agent can parse or read it as-is. |
| npm package name | `@jig-cli/mcp-server` | Scoped under `@jig-cli` to avoid name conflicts. Matches the pattern of `jig-cli` crate name on crates.io. |
| Location in repo | `mcp-server/` at repo root | Separate from the Rust crate (`src/`). Has its own `package.json`, `tsconfig.json`, and test suite. The jig binary is a runtime dependency found on PATH, not a build dependency. |
| Timeout | 30 seconds default | Recipes/workflows should complete quickly. 30s is generous. A stuck process shouldn't block the agent indefinitely. |

## Risks / Open Questions

- **Risk: jig binary version mismatch.** The MCP server wraps whatever `jig` is on PATH. If the CLI adds/changes a flag, the MCP server's argument builders might generate wrong flags. **Mitigation:** The MCP server captures the jig version at startup and logs it. If we later add a minimum version check, the version is already available.

- **Risk: `npx` startup latency.** First-time `npx` invocation downloads the package, which takes seconds. Subsequent runs use cache. **Mitigation:** Agents typically start the MCP server once per session. Document that first-run latency is expected; suggest local install (`npm install -g`) for faster starts.

- **Risk: Cross-platform PATH lookup.** `which` on macOS/Linux vs `where` on Windows. **Mitigation:** Use Node.js's `child_process.execSync('which jig')` which works on macOS/Linux. Windows support can be added later if needed (jig doesn't currently build for Windows).

- **Open question: Should the server accept `--timeout` as a CLI flag?** Leaning yes — different agents and recipe complexities may need different timeouts. Default 30s is reasonable, but make it configurable.

- **Open question: Should we add a `jig_version` resource (MCP resource, not tool)?** Would let agents check jig compatibility. Deferring — not needed for v1, and MCP resources are less universally supported than tools.
