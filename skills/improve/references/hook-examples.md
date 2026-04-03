# Hook Examples Reference

Hooks provide deterministic automation at specific points in Claude Code's lifecycle. Unlike skills (which Claude reasons about), hooks always execute when their conditions match.

## When to Use Hooks

| Use a Hook when | Use a Skill when |
|----------------|------------------|
| Action must happen every time (formatting, logging) | Action requires reasoning |
| Need to block dangerous operations | Need to adapt to context |
| Need to auto-approve safe operations | Need user interaction |
| Need to run before/after specific tools | Need multi-step workflow |

## Hook Configuration Location

Hooks go in `settings.json` under the `hooks` key:
- **Project**: `.claude/settings.json` (shared via git)
- **User**: `~/.claude/settings.json` (personal)
- **Skill-scoped**: In SKILL.md frontmatter (active only during skill)

## Hook Structure

```json
{
  "hooks": {
    "<EventName>": [
      {
        "matcher": "<regex matching tool name or event type>",
        "hooks": [
          {
            "type": "command",
            "command": "<shell command or script path>",
            "timeout": 600
          }
        ]
      }
    ]
  }
}
```

## Key Events

| Event | Fires when | Can block? | Best for |
|-------|-----------|-----------|----------|
| `PreToolUse` | Before tool executes | Yes (exit 2) | Guardrails, validation |
| `PostToolUse` | After tool succeeds | No | Formatting, logging |
| `PermissionRequest` | Permission dialog appears | Yes | Auto-approve/deny |
| `UserPromptSubmit` | User sends a message | Yes | Prompt validation |
| `Stop` | Claude finishes responding | Yes | Completion verification |
| `Notification` | Claude needs attention | No | Desktop alerts |
| `SessionStart` | Session begins | No | Context injection |

## Common Matchers

```
"Bash"              # Match Bash tool
"Edit|Write"        # Match Edit or Write
"mcp__github__.*"   # Match all GitHub MCP tools
"Bash(git *)"       # Match only git commands (use in "if" field)
```

## Hook Input (JSON on stdin)

```json
{
  "session_id": "abc123",
  "cwd": "/path/to/project",
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf /" }
}
```

## Hook Output

- **Exit 0**: Allow. Stdout text becomes context for Claude.
- **Exit 2**: Block. Stderr becomes feedback to Claude.
- **JSON output** (optional): Structured control over the decision.

---

## Example 1: Block Destructive Commands

**Problem**: Prevent accidental `rm -rf`, `DROP TABLE`, force pushes.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "INPUT=$(cat); CMD=$(echo \"$INPUT\" | jq -r '.tool_input.command // empty'); if echo \"$CMD\" | grep -qE '(rm -rf|DROP TABLE|--force|--hard)'; then echo \"Blocked: destructive command\" >&2; exit 2; fi; exit 0"
          }
        ]
      }
    ]
  }
}
```

For complex logic, use a script file instead of inline commands.

## Example 2: Auto-Format After Edits

**Problem**: Keep files formatted without Claude remembering to run the formatter.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "INPUT=$(cat); FILE=$(echo \"$INPUT\" | jq -r '.tool_input.file_path // empty'); if [ -n \"$FILE\" ] && echo \"$FILE\" | grep -qE '\\.(ts|tsx|js|jsx|vue)$'; then npx prettier --write \"$FILE\" 2>/dev/null; fi; exit 0"
          }
        ]
      }
    ]
  }
}
```

## Example 3: Auto-Approve Safe Permissions

**Problem**: Stop getting prompted for read-only operations.

```json
{
  "hooks": {
    "PermissionRequest": [
      {
        "matcher": "Read|Grep|Glob",
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"hookSpecificOutput\": {\"hookEventName\": \"PermissionRequest\", \"decision\": {\"behavior\": \"allow\"}}}'"
          }
        ]
      }
    ]
  }
}
```

## Example 4: Desktop Notification (macOS)

**Problem**: Get notified when Claude needs input while you're in another app.

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "osascript -e 'display notification \"Claude needs your attention\" with title \"Claude Code\"'"
          }
        ]
      }
    ]
  }
}
```

## Example 5: Audit Logging

**Problem**: Log all tool usage for compliance or debugging.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "INPUT=$(cat); echo \"$(date -u +%Y-%m-%dT%H:%M:%SZ) $(echo $INPUT | jq -c '{tool: .tool_name, input: .tool_input}')\" >> /tmp/claude-audit.log; exit 0",
            "async": true
          }
        ]
      }
    ]
  }
}
```

Note `"async": true` — logging doesn't block Claude's workflow.

## Example 6: Skill-Scoped Hook

**Problem**: Only enforce a guardrail during a specific skill.

In the skill's `SKILL.md` frontmatter:

```yaml
---
name: careful-deploy
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "INPUT=$(cat); CMD=$(echo \"$INPUT\" | jq -r '.tool_input.command // empty'); if echo \"$CMD\" | grep -qE '(rm|drop|delete|force)'; then echo 'Blocked during deploy' >&2; exit 2; fi"
---
```

This hook only fires while `/careful-deploy` is active.

## Writing Hook Scripts

For anything beyond a one-liner, put the logic in a script:

```bash
#!/bin/bash
# .claude/hooks/my-hook.sh
INPUT=$(cat)
TOOL=$(echo "$INPUT" | jq -r '.tool_name')
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Your logic here

exit 0  # Allow (or exit 2 to block)
```

Make it executable: `chmod +x .claude/hooks/my-hook.sh`

Reference from settings.json: `"command": ".claude/hooks/my-hook.sh"`
