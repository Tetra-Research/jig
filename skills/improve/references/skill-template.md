# Skill Template Reference

Use this as a starting point when scaffolding a new skill.

## Directory Structure

```
<skill-name>/
├── SKILL.md              # Required: instructions + frontmatter
├── references/           # Optional: supporting docs Claude reads on demand
│   ├── api.md
│   └── examples.md
├── scripts/              # Optional: executable scripts Claude can run
│   └── validate.sh
└── templates/            # Optional: output templates to copy
    └── output.md
```

Most skills only need `SKILL.md`. Add folders only when needed.

## SKILL.md Template

```yaml
---
name: <kebab-case-name>
description: <What this does and when to use it. Under 250 chars. Front-load trigger words.>
---

# <Skill Title>

<1-2 sentence overview of what this skill does.>

## Steps

### Step 1: <Gather context>
<What to read, ask, or look up before acting.>

### Step 2: <Do the work>
<The core instructions.>

### Step 3: <Verify>
<How to confirm the output is correct.>

## Gotchas
- <Common mistake Claude makes with this workflow>
- <Edge case to watch for>
```

## Frontmatter Fields

Only include fields you actually need. The defaults are sensible.

### Required
- **`name`**: Becomes the `/slash-command`. Lowercase, hyphens only. Max 64 chars.
- **`description`**: When Claude should use this skill. This is a trigger condition, not a summary.

### Common Optional Fields
- **`allowed-tools`**: Space-separated tool names Claude can use without prompting. Example: `Read Grep Glob Bash(git *)`
- **`disable-model-invocation`**: Set `true` for skills with side effects (deploy, post, delete). Only user can invoke via `/name`.
- **`argument-hint`**: Shown in autocomplete. Example: `[filename] [format]`

### Advanced Fields (use sparingly)
- **`context: fork`**: Run in an isolated subagent. Good for heavy read-only analysis.
- **`agent`**: Which subagent type when `context: fork`. Example: `Explore`, `Plan`, or a custom agent name.
- **`model`**: Override model. Example: `haiku` for fast exploration, `opus` for deep reasoning.
- **`effort`**: Override effort level: `low`, `medium`, `high`, `max`.
- **`paths`**: Glob patterns — auto-load when working with matching files. Example: `src/**/*.ts`
- **`user-invocable: false`**: Hide from `/` menu. Only Claude can use it (background knowledge skills).
- **`hooks`**: Skill-scoped hooks that fire only when this skill is active. See hook-examples.md.

## Dynamic Content

### Arguments
- `$ARGUMENTS` — everything after `/skill-name`
- `$ARGUMENTS[0]` or `$0` — first argument
- `$ARGUMENTS[1]` or `$1` — second argument

### Inline Commands (run before Claude sees the prompt)
```markdown
Current branch: !`git branch --show-current`
Changed files: !`git diff --name-only`
```

### Skill Directory Reference
```markdown
See ${CLAUDE_SKILL_DIR}/references/api.md for details.
```

## Description Writing Guide

The description determines auto-invocation. Write it like a trigger condition:

**Good** (front-loads when to use):
```
Review code for security vulnerabilities, performance issues, and style violations. Use when auditing code or before merging PRs.
```

**Bad** (generic summary):
```
A helpful skill that reviews code and provides feedback.
```

## Plugin vs Project Placement

| Put it here | When |
|------------|------|
| `.claude/skills/<name>/SKILL.md` | Repo-specific workflow |
| `~/.claude/skills/<name>/SKILL.md` | Personal workflow, all repos |
