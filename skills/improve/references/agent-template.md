# Agent Template Reference

Use this when the user needs a specialized role with tool restrictions or a different model.

## When to Use an Agent vs a Skill

| Use an Agent when | Use a Skill when |
|-------------------|------------------|
| Need restricted tools (read-only, no bash) | Full tool access is fine |
| Need a different model (haiku for speed, opus for depth) | Default model works |
| Role is reusable across many skills | Workflow is a one-off |
| Want to run in isolated context from Agent tool | Runs inline in conversation |

## Agent Definition File

Location: `.claude/agents/<name>.md` (project) or `~/.claude/agents/<name>.md` (personal)

### Template

```markdown
---
name: <kebab-case-name>
description: <When to use this agent. Written for the model, not humans.>
model: <sonnet|opus|haiku|inherit>
tools:
  - Read
  - Grep
  - Glob
---

<System prompt defining the agent's expertise, personality, and approach.>

<Instructions for how it should work, what to focus on, what to avoid.>
```

## Frontmatter Fields

### Required
- **`name`**: Agent identifier. Used in `subagent_type` parameter of Agent tool.
- **`description`**: When Claude should delegate to this agent. Front-load trigger conditions.

### Optional
- **`model`**: `haiku` (fast/cheap), `sonnet` (balanced), `opus` (deep reasoning), `inherit` (match parent). Default: inherit.
- **`tools`**: Array of allowed tools. Omit to inherit all parent tools.
- **`permissions`**: Permission mode: `dontAsk`, `plan`, `acceptEdits`. Default: inherits.

## Tool Restriction Patterns

```yaml
# Read-only analysis (no modifications)
tools:
  - Read
  - Grep
  - Glob

# Code modification (no shell access)
tools:
  - Read
  - Edit
  - Write
  - Grep
  - Glob

# Shell + read (test runner, build checker)
tools:
  - Bash
  - Read
  - Grep

# Full access (omit tools field entirely)
```

## Model Selection Guide

| Model | Use for | Cost | Speed |
|-------|---------|------|-------|
| **haiku** | File discovery, simple lookups, fast exploration | Lowest | Fastest |
| **sonnet** | Most tasks, good balance of capability and cost | Medium | Medium |
| **opus** | Complex reasoning, security reviews, architecture decisions | Highest | Slowest |
| **inherit** | Match whatever the user's session is using | Varies | Varies |

## Example: Read-Only Code Reviewer

```markdown
---
name: security-reviewer
description: Review code for security vulnerabilities including injection, auth bypass, data exposure, and OWASP Top 10. Use when auditing sensitive code paths.
model: opus
tools:
  - Read
  - Grep
  - Glob
---

You are a security-focused code reviewer. Your job is to find vulnerabilities, not style issues.

Focus on:
- SQL injection (raw queries, f-strings with user input)
- XSS (v-html with user content, unsanitized output)
- Auth bypass (missing permission checks, exposed endpoints)
- Data exposure (PII in logs, secrets in code, unencrypted sensitive fields)
- Command injection (subprocess with shell=True, unsanitized args)

For each finding, report:
1. File and line number
2. Vulnerability type
3. Severity (critical/high/medium/low)
4. Concrete fix

Be thorough but avoid false positives. If you're unsure, say so.
```

## Example: Fast Explorer

```markdown
---
name: codebase-scout
description: Quickly explore and map unfamiliar parts of the codebase. Use when you need to understand structure, find patterns, or locate code.
model: haiku
tools:
  - Read
  - Grep
  - Glob
---

You are a fast codebase explorer. Your job is to find things quickly and report back concisely.

When asked to find something:
1. Start with Glob to find candidate files by name/pattern
2. Use Grep to search content across matches
3. Read relevant sections (not whole files)
4. Report what you found with file paths and line numbers

Keep responses short. List findings, don't explain obvious things.
```

## How Agents Get Invoked

1. **By the Agent tool**: Claude (or a skill) spawns the agent via the Agent tool with `subagent_type: "<name>"`
2. **By skill delegation**: A skill with `context: fork` and `agent: <name>` runs inside that agent
3. **By description match**: When Claude decides a task matches the agent's description

Agents run in isolated context — they don't see the parent conversation history. They receive only their system prompt and the task prompt.
