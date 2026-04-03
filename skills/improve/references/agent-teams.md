# Agent Teams Reference

Agent teams coordinate multiple Claude Code instances working in parallel on the same codebase. Unlike sub-agents (which report back to a parent), teammates communicate peer-to-peer, claim work from a shared task list, and self-organize.

**Status**: Experimental. Requires `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`.

## When to Use Agent Teams

**Strong cases:**
- Parallel code review from different angles (security, performance, test coverage)
- Competing hypotheses — each teammate investigates a different theory, then they debate
- Multi-module implementation where each teammate owns a different set of files
- Large-scale QA (one teammate per test dimension)

**Don't use when:**
- Tasks are sequential (coordination overhead > benefit)
- Multiple teammates need to edit the same files (overwrites)
- A single sub-agent could handle it (cheaper)
- The task is simple enough for one session

## Enabling Teams

```json
// .claude/settings.json or ~/.claude/settings.json
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  }
}
```

## Spawning Teammates

Ask the lead (your main session) in natural language:

```
Create a team with 3 teammates:
- "security-reviewer" focused on auth, XSS, CSRF, input validation
- "perf-reviewer" focused on N+1 queries, memory, expensive ops
- "test-reviewer" focused on coverage, edge cases, integration tests

Have them each review the changes on this branch independently.
```

**Key facts about spawning:**
- Teammates don't inherit the lead's conversation history — only project context (CLAUDE.md, MCP, skills) and the spawn prompt
- Include task-specific details in the spawn prompt
- All teammates start with the lead's permission mode and model
- You can reference agent definitions from `.claude/agents/` by name when spawning

## Communication

### Lead to Teammate
The lead sends messages via `SendMessage(to="teammate-name", message="...")`.

### Teammate to Teammate
Teammates can message each other directly. Messages are delivered automatically via a mailbox system.

### Broadcast
Send to all teammates at once. Use sparingly — cost scales linearly with team size.

### Shared Task List
- Tasks live at `~/.claude/tasks/{team-name}/`
- Teammates self-claim unassigned, unblocked tasks
- Tasks can have dependencies (blocked until prerequisite completes)
- Lead can assign tasks explicitly

## Display Modes

| Mode | How it works | Requirements |
|------|-------------|-------------- |
| **in-process** | All teammates in one terminal. Shift+Down to cycle. | None (works anywhere) |
| **tmux** | Each teammate in its own tmux pane. | tmux installed |
| **auto** | Detects tmux/iTerm2, falls back to in-process. | Default |

**Navigation (in-process mode):**
- `Shift+Down` — cycle to next teammate
- `Ctrl+T` — toggle task list
- `Escape` — interrupt current teammate
- `Enter` — view teammate's full session

## Hub-and-Spoke Pattern

For structured debates or multi-perspective investigation:

```
              Lead (orchestrator)
              │
    ┌─────────┼─────────┐
    │         │         │
 sceptic  analyst   debater
 (child)  (child)   (child)
```

**How it works:**
1. Lead spawns teammates with role-specific prompts + shared context
2. Round 1: Each teammate forms an independent position
3. Lead collects findings and relays messages between teammates
4. Round 2+: Teammates respond to each other's arguments
5. Lead synthesizes consensus from the debate

## Worktree Isolation

For implementation work where teammates modify files, use git worktrees:

```
Spawn 3 teammates, each in their own worktree:
- "frontend" working on Vue components
- "backend" working on Django views
- "tests" writing integration tests

Each teammate should commit to their own branch. We'll merge when done.
```

Each worktree gets an independent branch, eliminating file conflict risk.

## Quality Gate Hooks

### Prevent premature task completion
```json
{
  "hooks": {
    "TaskCompleted": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Verify tests pass before marking complete' >&2; exit 2"
          }
        ]
      }
    ]
  }
}
```

### Keep teammates productive (prevent idle)
```json
{
  "hooks": {
    "TeammateIdle": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Check the task list for remaining work' >&2; exit 2"
          }
        ]
      }
    ]
  }
}
```

## Cost Management

- **Token usage scales linearly** with team size — each teammate is a full context window
- **Sweet spot**: 3-5 teammates with 5-6 tasks each
- **Use Sonnet for teammates** (balances capability and cost)
- **Plan-first**: Identify task breakdown in plan mode (cheap), then execute with team (expensive but parallel)
- **Clean up promptly**: Active teammates consume tokens even when idle

## Limitations (Experimental)

| Limitation | Workaround |
|-----------|-----------|
| No session resumption — `/resume` doesn't restore teammates | Spawn new teammates after resuming |
| One team per session | Clean up before starting a new team |
| No nested teams — teammates can't spawn sub-teams | Orchestrate everything from the lead |
| Lead role is fixed — can't promote a teammate | Plan for this when designing the team |
| Task status can lag — teammates sometimes forget to mark tasks done | Manually nudge or update via the lead |
| File conflicts if teammates edit the same file | Use worktrees or assign file ownership |
| Split panes require tmux or iTerm2 | Use in-process mode as fallback |
