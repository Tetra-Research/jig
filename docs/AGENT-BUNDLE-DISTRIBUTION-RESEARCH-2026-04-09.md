# Agent Bundle Distribution Research - 2026-04-09

Temporary working note before compaction.

## Goal

Figure out how Jig should distribute agent integrations for:

- Claude Code
- Codex
- OpenCode

Focus of this note:

- how plugins and skills are hosted today
- whether Jig needs a hosted service
- what release/distribution shape best fits Jig
- what install UX we want in the CLI

## Repo Grounding

Jig already points toward skill-local packaging rather than a central registry:

- `README.md` says Jig is "designed for skill-local ownership" and shows recipes/workflows living inside the skill that uses them.
- `src/main.rs` already scans agent skill directories, including `.claude/skills` and `.codex/skills`.
- `PRD.md` explicitly says Jig is not Claude-specific and should work across Claude Code, Codex, OpenCode, and others via the CLI first.
- `docs/ROADMAP.md` already reserves a deeper plugin milestone for Claude Code.

Relevant repo references:

- [README.md](/Users/tylerobriant/code/tetra/jig/README.md#L5)
- [README.md](/Users/tylerobriant/code/tetra/jig/README.md#L103)
- [src/main.rs](/Users/tylerobriant/code/tetra/jig/src/main.rs#L103)
- [src/main.rs](/Users/tylerobriant/code/tetra/jig/src/main.rs#L1148)
- [PRD.md](/Users/tylerobriant/code/tetra/jig/PRD.md#L2002)
- [PRD.md](/Users/tylerobriant/code/tetra/jig/PRD.md#L2182)
- [PRD.md](/Users/tylerobriant/code/tetra/jig/PRD.md#L2281)
- [docs/ROADMAP.md](/Users/tylerobriant/code/tetra/jig/docs/ROADMAP.md#L153)

## External Research Summary

### Claude Code

Claude Code now has a real plugin marketplace model.

Current documented distribution paths:

- official Anthropic marketplace
- custom marketplace from GitHub
- custom marketplace from any git URL
- custom marketplace from a local path
- custom marketplace from a hosted `marketplace.json` URL

Important implications:

- Claude plugins can bundle skills, agents, hooks, MCP servers, and LSP servers.
- Anthropic recommends GitHub hosting for marketplaces.
- Relative plugin paths work best when the marketplace is added from git, not from a raw hosted JSON URL.
- Users add a marketplace first, then install individual plugins from it.

Conclusion for Jig:

- We do not need to host a remote service just to distribute Claude skills.
- A git-hosted marketplace repo is enough for public or team distribution.
- A repo-local plugin is enough for local/project use.

Sources:

- https://code.claude.com/docs/en/discover-plugins
- https://code.claude.com/docs/en/plugin-marketplaces
- https://code.claude.com/docs/en/plugins
- https://code.claude.com/docs/en/mcp
- https://code.claude.com/docs/en/slash-commands

### OpenCode

OpenCode does not use the same kind of first-party marketplace model as Claude Code.

Current documented distribution paths:

- local plugins from `.opencode/plugins/` or `~/.config/opencode/plugins/`
- npm-distributed plugins declared in config
- repo-local skills from `.opencode/skills/`
- compatibility loading from `.claude/skills/` and `.agents/skills/`

Important implications:

- OpenCode skills are already easy to ship in-repo without a plugin package.
- OpenCode plugins are usually code packages, not marketplace entries.
- npm is the main portable distribution path if we want a reusable plugin package.
- MCP servers can be local or remote, but they are separate from plugin packaging.

Conclusion for Jig:

- We do not need hosted infrastructure for OpenCode if we are only shipping skills.
- If we later want plugin hooks or runtime logic, npm is the natural distribution channel.

Sources:

- https://opencode.ai/docs/plugins/
- https://opencode.ai/docs/skills/
- https://opencode.ai/docs/mcp-servers/
- https://opencode.ai/docs/ecosystem/

### Codex

Codex now has plugin support and an official plugin directory, but the self-serve public publishing story is not fully open yet.

Current documented distribution paths:

- local plugin folders with `.codex-plugin/plugin.json`
- repo-local marketplace file at `.agents/plugins/marketplace.json`
- home-local marketplace file at `~/.agents/plugins/marketplace.json`
- bundled skills, apps, and MCP inside plugins

Important implications:

- Codex plugin packaging is real and documented.
- Local marketplace and local plugin flows are documented and usable now.
- The docs point to curated plugin browsing in the app, but for our purposes local and repo-scoped distribution is the practical path.
- Codex also supports MCP directly through config, so typed tool access is independent from plugin packaging.

Conclusion for Jig:

- We do not need to host a remote service just to install Codex skills/plugins.
- Repo-local marketplace + plugin bundles is enough for the first launch.
- Public marketplace distribution can wait until OpenAI's self-serve story is clearer.

Sources:

- https://developers.openai.com/codex/plugins
- https://developers.openai.com/codex/plugins/build
- https://developers.openai.com/codex/skills
- https://developers.openai.com/codex/mcp

## Core Conclusion

For Jig's first cross-agent launch, "hosting plugins" does not mean "run a hosted backend."

The practical distribution unit is:

- versioned files in the Jig release
- installed into agent-specific folders by the `jig` CLI

Only build or host a remote service when we actually need remote MCP functionality.

That means:

- skills and plugin metadata should ship inside release bundles
- MCP server hosting is a separate decision
- the binary installer and the agent integration installer should be distinct concerns

## Product Decision

Ship versioned agent bundles inside each Jig release.

The release should contain:

- the `jig` binary
- agent bundle payloads for Claude Code, Codex, and OpenCode

The install flow should be:

1. `install.sh` installs the `jig` binary only.
2. `jig` installs agent assets from the same release payload.

Why this split is better:

- `install.sh` stays narrow, verifiable, and low-risk.
- Agent-specific path logic lives in one place: the Rust CLI.
- Binary version and skill/plugin version stay locked together.
- We can add update, remove, doctor, and list commands later without changing the shell installer.

Relevant repo references:

- [install.sh](/Users/tylerobriant/code/tetra/jig/install.sh#L1)
- [src/library/install.rs](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L8)

## Recommended Command Shape

New command family:

```bash
jig agent install [agent]
jig agent update [agent]
jig agent remove [agent]
jig agent list
jig agent doctor [agent]
```

Recommended `install` behavior:

```bash
jig agent install claude
jig agent install codex
jig agent install opencode
```

Optional flags:

```bash
jig agent install claude --scope project
jig agent install claude --scope user
jig agent install codex --to /path/to/repo
jig agent install --agent codex
jig agent install --all
```

## Default Agent Selection

Desired UX:

- if the user explicitly passes an agent, use it
- otherwise, infer the current agent from the environment or repo and install into that target

Recommended precedence:

1. explicit `--agent` or positional agent
2. known runtime environment markers, if available
3. repo-local config markers
4. fail with a clear ambiguity error instead of guessing silently

Recommended repo marker detection:

- Claude Code:
  - `.claude/`
  - `.claude/skills/`
  - `.claude-plugin/`
- Codex:
  - `.codex/`
  - `.agents/plugins/marketplace.json`
  - `.codex-plugin/`
- OpenCode:
  - `.opencode/`
  - `.opencode/skills/`
  - `.opencode/plugins/`

If multiple agent markers are present:

- do not guess
- print a short error listing the detected agents
- require `--agent`

This is safer than silently writing to the wrong config tree.

## Recommended On-Disk Install Targets

### Claude Code

Two plausible install modes:

- lightweight:
  - install skills into `.claude/skills/jig-*`
- full plugin:
  - install plugin bundle into a plugin folder
  - update or seed the Claude marketplace/plugin config for the chosen scope

Recommendation:

- start with skill-only install for the earliest path
- keep plugin-bundle support in the release layout so we can grow into marketplace mode cleanly

### Codex

Two plausible install modes:

- lightweight:
  - install skills into `.codex/skills/` when that is sufficient
- full plugin:
  - install plugin folder with `.codex-plugin/plugin.json`
  - update `.agents/plugins/marketplace.json`

Recommendation:

- prefer full plugin install for Codex because the packaging model is already local-marketplace based
- still keep skill-only mode available for local development and tests

### OpenCode

Two plausible install modes:

- lightweight:
  - install skills into `.opencode/skills/`
- plugin:
  - install plugin code into `.opencode/plugins/`

Recommendation:

- start with skills only
- add plugin mode only if we need hooks, runtime behavior, or custom tools

## Bundle Structure Inside The Release

Recommended release asset shape after unpack:

```text
jig
bundles/
  claude/
    skills/
    plugin/
  codex/
    skills/
    plugin/
  opencode/
    skills/
    plugin/
```

Alternative:

```text
share/jig/agents/<agent>/...
```

Either is fine. The key requirement is that the `jig` binary can locate the bundle assets from the installed package without network access.

## Update Model

Agent assets should update the same way libraries do:

- install copies versioned content into the target path
- install writes metadata describing:
  - Jig version
  - agent kind
  - source bundle
  - scope
  - install timestamp
- update can then refresh from the currently installed Jig release

This is parallel to how library install metadata already works in `src/library/install.rs`.

## Why Not Put Agent Install Logic In `install.sh`

Do not make `install.sh` directly manage Claude/Codex/OpenCode directories.

Reasons:

- shell logic will become fragile fast
- target paths differ by agent and scope
- update/remove/list/doctor would have to be duplicated elsewhere
- the shell installer should remain a minimal trusted bootstrapper

The shell installer should stay responsible for:

- fetching the release
- verifying signatures and checksums
- installing the binary

Then the binary handles everything higher-level.

## Open Questions

These need to be answered in the implementation spec:

1. Should `jig agent install claude` install a pure skill layout first, or a full Claude plugin immediately?
2. For Codex, should the default be skill-only or marketplace-backed plugin install?
3. Should the release ship one cross-agent "jig helper" bundle, or agent-specific bundles with different naming and docs?
4. Do we want one logical Jig skill per workflow, or a smaller number of broader entrypoint skills?
5. How should `jig agent install` behave in repos that already contain user-owned skills under the same path?
6. Should agent install be project-local by default, with `--scope user` opt-in?

## Current Recommendation

Ship this in phases:

### Phase 1

- `install.sh` keeps installing only the binary
- release bundles include `claude`, `codex`, and `opencode` assets
- new CLI command: `jig agent install`
- default install target:
  - explicit agent if passed
  - otherwise infer from the current repo markers
  - otherwise fail clearly

### Phase 2

- add `jig agent update`, `remove`, `list`, and `doctor`
- add plugin-mode install where it is worth it
- keep MCP server setup separate unless bundling it materially improves UX

### Phase 3

- decide whether Claude marketplace publishing should be first-class
- decide whether OpenCode npm publishing is worth doing
- revisit Codex public marketplace support when the self-serve story is clearer

## Bottom Line

The right first launch shape is:

- keep agent assets in this repo
- ship them inside versioned Jig releases
- install them from the `jig` CLI, not from the shell installer
- default to the current agent environment when it is unambiguous
- avoid building hosted infrastructure unless we actually need remote MCP
