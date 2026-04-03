# Extension Points Decision Tree

Use this to determine which Claude Code extension point fits the user's problem.

## Decision Flow

```
What's the problem?
│
├─ "Claude keeps doing X wrong"
│   ├─ Applies to specific file types → PATH-SPECIFIC RULE (.claude/rules/<name>.md)
│   ├─ Applies everywhere in this repo → CLAUDE.md addition
│   └─ Only during a specific workflow → SKILL (with gotchas section)
│
├─ "I do this multi-step process repeatedly"
│   ├─ Has side effects (deploy, post, delete) → SKILL (disable-model-invocation: true)
│   └─ No side effects → SKILL (auto-invocable)
│
├─ "I need Claude to act as an expert in X"
│   ├─ Needs restricted tools (read-only, no bash) → AGENT
│   ├─ Needs a different model (haiku for speed, opus for depth) → AGENT
│   └─ Just needs domain knowledge → SKILL or CLAUDE.md
│
├─ "I want to block/validate/auto-approve actions"
│   ├─ Block destructive commands → HOOK (PreToolUse, exit 2 to block)
│   ├─ Auto-format after edits → HOOK (PostToolUse on Edit|Write)
│   ├─ Auto-approve safe operations → HOOK (PermissionRequest)
│   ├─ Log all actions for audit → HOOK (PostToolUse, async)
│   └─ Only during specific skill → SKILL with embedded hooks
│
├─ "Claude needs to query an external system"
│   ├─ System has an MCP server available → MCP SERVER (.claude/.mcp.json)
│   ├─ System has a REST API → MCP SERVER (build or use generic HTTP MCP)
│   └─ System has a CLI tool → SKILL (with Bash tool access)
│
├─ "I need multiple agents working in parallel"
│   ├─ Independent modules, no shared files → AGENT TEAM (worktree per teammate)
│   ├─ Competing hypotheses / debate → AGENT TEAM (hub-and-spoke pattern)
│   ├─ Parallel review (security + perf + tests) → AGENT TEAM (3-5 reviewers)
│   └─ Sequential pipeline or same-file edits → DON'T use teams, use single agent
│
├─ "I want to bundle multiple extensions"
│   └─ PLUGIN (backend/shared/claude-plugins/<name>/)
│
└─ "Claude should remember X across sessions"
    ├─ About the user (role, preferences) → MEMORY (user type)
    ├─ About the project (decisions, deadlines) → MEMORY (project type)
    └─ About how to work (corrections, confirmations) → MEMORY (feedback type)
```

## Extension Point Comparison

| Extension | Loads when | Can block actions | Has scripts | Shared via git | Auto-triggers |
|-----------|-----------|-------------------|-------------|---------------|---------------|
| Skill | User invokes `/name` or Claude matches description | No | Yes | Yes | By description match |
| Agent | Parent Agent tool specifies type | No | No | Yes | By description match |
| Hook | Lifecycle event fires + matcher matches | Yes (PreToolUse) | Yes | Yes | Always (when matched) |
| Rule | File matching glob pattern is accessed | No | No | Yes | By path pattern |
| CLAUDE.md | Every session start | No | No | Yes | Always |
| MCP Server | Session start (connects) | No | N/A | Yes (.mcp.json) | Tools available always |
| Plugin | When installed + enabled | Inherits | Yes | Via marketplace | Inherits |
| Agent Team | Lead spawns teammates | No | No | Via settings.json env | Lead orchestrates |
| Memory | Session start (first 200 lines) | No | No | No (machine-local) | Always loaded |

## Scope Guidance

| Scope | Where to put it | Who sees it |
|-------|----------------|-------------|
| Just me | `~/.claude/skills/`, `~/.claude/agents/` | Only you |
| This repo | `.claude/skills/`, `.claude/agents/`, `.claude/rules/` | Everyone on this repo |
| Multiple repos | Plugin directory | Anyone who installs the plugin |
| Org-wide enforcement | Managed settings (admin) | Everyone in the org |

## Common Combinations

- **Skill + Hook**: Skill provides the workflow, hook provides guardrails during that workflow (use skill-level hooks in frontmatter)
- **Skill + Agent**: Skill orchestrates the workflow, delegates to specialized agent for a sub-task (use `context: fork` + `agent: <name>`)
- **Skill + MCP**: Skill needs external data, MCP provides the connection
- **Rule + Hook**: Rule provides instructions for file type, hook enforces them deterministically
- **Agent + MCP**: Agent needs specialized tools scoped to its role
- **Agent Team + Hook**: TeammateIdle hook prevents idle teammates; TaskCompleted hook enforces quality gates before a teammate can mark work done
- **Agent Team + Worktree**: Each teammate gets an isolated git worktree — eliminates file conflict risk for parallel implementation work
- **Agent Team + Skill**: Teammates load the same skills as the lead — a skill can be invoked by any teammate during their work

## Agent Teams vs Sub-Agents

| | Sub-Agent | Agent Team |
|--|-----------|------------|
| **Communication** | Reports back to parent only | Peer-to-peer messaging between teammates |
| **Context** | Fresh context, returns summary | Each teammate maintains full independent context |
| **Coordination** | Parent orchestrates | Shared task list, self-organizing |
| **Cost** | Cheap (summary returns) | Expensive (N full context windows) |
| **Use when** | Delegating a single bounded task | Parallel work that benefits from debate or independence |
| **Experimental?** | Stable | Experimental (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`) |
