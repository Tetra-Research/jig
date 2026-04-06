# CLAUDE.md

**Always check for a matching skill before hand-editing files.** This project has skills (in `.claude/skills/`) that handle common tasks like adding views, endpoints, and models. Use them — they're faster and produce consistent output.

Run `jig list --skills --claude --json` to discover available skills before reading individual SKILL.md files.

If a skill uses jig and jig fails (non-zero exit), it prints rendered content in stderr. Apply that output manually.
