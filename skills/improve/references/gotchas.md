# Gotchas & Lessons Learned

A living document of non-obvious lessons discovered while building Claude Code extensions. Add to this whenever you discover something surprising.

---

## Skills

- **Description is the trigger, not a summary.** If your skill isn't auto-triggering, rewrite the description to front-load the words a user would naturally say. "Review code for bugs" triggers on "review my code". "A helpful code review skill" triggers on nothing.

- **Keep SKILL.md under 200 lines.** Long skills consume context even when not invoked (the description is always loaded). Move reference material to separate files in `references/`.

- **`$ARGUMENTS` is empty when auto-invoked.** If Claude triggers a skill based on description match (not `/slash-command`), `$ARGUMENTS` will be empty. Design skills to work both ways.

- **Don't duplicate CLAUDE.md content in skills.** Claude already has CLAUDE.md loaded. Skills that repeat project conventions waste context and can contradict if CLAUDE.md changes.

- **Skill names can't contain dots or underscores.** Only lowercase letters, numbers, and hyphens. `my-skill` works, `my_skill` and `my.skill` don't.

## Agents

- **Agents don't see parent conversation.** They start fresh with only their system prompt and the task prompt. If context from the conversation is needed, include it explicitly in the Agent tool's prompt parameter.

- **Don't restrict tools you forgot about.** If an agent needs to read files, it probably also needs `Glob` to find them. Common missing tool: `Glob` when you listed `Read` and `Grep`.

- **Haiku agents are cheap but miss nuance.** Use haiku for file discovery and pattern matching. Don't use it for security reviews or architecture decisions — it will miss things.

## Hooks

- **Shell profiles can break hooks.** If your `.zshrc` or `.bashrc` has unconditional `echo` statements, they'll corrupt hook JSON output. Wrap in `if [[ $- == *i* ]]; then ... fi`.

- **PreToolUse exit 2 blocks AND gives feedback.** Whatever you write to stderr becomes context for Claude. Use this to explain *why* something was blocked so Claude can adjust.

- **PostToolUse can't block.** It fires after the tool already succeeded. If you need to prevent an action, use PreToolUse.

- **Async hooks don't block but also don't provide feedback.** Use `"async": true` only for logging/telemetry where you don't need Claude to see the result.

- **Stop hooks can infinite-loop.** If a Stop hook blocks (exit 2), Claude tries again, which fires the hook again. Check the `stop_hook_active` field and exit 0 if true.

## Rules & CLAUDE.md

- **Rules load lazily.** A rule with `paths: "src/**/*.ts"` only loads when Claude accesses a matching file. Don't expect it to apply to all TypeScript files from session start.

- **CLAUDE.md loads eagerly.** Everything in CLAUDE.md consumes context from the start. Keep it under 200 lines. Move detailed reference material to skills or rules.

- **Rules override CLAUDE.md on conflict.** If CLAUDE.md says "use 4 spaces" and a rule for `*.py` says "use 2 spaces", the rule wins for Python files.

## Plugins

- **Plugin skills are namespaced.** A skill `review` in plugin `factory` becomes `/factory:review` (or just `/review` if no conflict).

- **`${CLAUDE_PLUGIN_ROOT}` changes on updates.** Don't hardcode paths to plugin files. Always use the variable.

- **Plugin data survives updates, plugin files don't.** Store persistent data in `${CLAUDE_PLUGIN_DATA}`, not in the plugin directory itself.

## Agent Teams

- **Teammates don't inherit conversation history.** They start fresh with only project context + spawn prompt. If a teammate needs context from your conversation, include it explicitly in the spawn prompt.

- **File conflicts are silent.** Two teammates editing the same file = last write wins, no merge. Either use worktrees or assign strict file ownership per teammate.

- **No session resumption.** `/resume` doesn't restore in-process teammates. After resuming, you need to spawn a new team.

- **One team per session.** You must clean up the current team before starting a new one. `TeamDelete` blocks if a teammate is hung.

- **Task status lags.** Teammates sometimes forget to mark tasks complete, blocking dependent work. Check manually and nudge from the lead.

- **Lead sometimes does the work itself.** The lead may implement tasks instead of delegating to teammates. If you notice this, explicitly tell the lead to wait for teammates.

- **Cost scales linearly.** 4 teammates = ~4x token usage. Justify teams only when parallelism or independent perspectives are worth the cost. For most tasks, a single session with sub-agents is cheaper.

- **Split panes need tmux.** In-process mode (Shift+Down to cycle) works everywhere. Split panes require tmux or iTerm2 with Python API.

## MCP Servers

- **MCP tools are deferred by default.** Claude only loads their full schemas when it decides to use them. This saves context but means Claude might not discover a useful tool. Write good tool descriptions on the server side.

- **MCP server failures are silent.** If a server fails to connect, Claude won't mention it unless you check. Use `/mcp` to verify connection status.

---

*Last updated: 2026-04-02. Add new lessons below this line.*
