---
name: discover
description: Discover available jig recipes and skills. Use before reading individual SKILL.md files, when deciding which jig recipe to use, or when exploring what jig can do in this project.
---

# Discover Jig Skills

Find the right jig skill for a task without reading every SKILL.md file.

## Steps

1. Run `jig list --skills --claude --json` to get a compact index of all available skills.
2. Match the task to a skill by name and description.
3. Read only that skill's SKILL.md for full instructions and variables.

## Gotchas

- **Do NOT read every SKILL.md file sequentially.** That wastes context. Use `jig list` first — it returns the full index in ~200 tokens.
- If `jig list` shows no skills, fall back to `ls .claude/skills/` and read individual files.
- The `path` field in the JSON output is the relative path to the skill directory. Read `<path>/SKILL.md` for full instructions.
