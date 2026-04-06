# CLAUDE.md

**This project uses `jig` to generate code.** Do not hand-write boilerplate — use jig recipes via skills instead. Skills in `.claude/skills/` wrap jig recipes for common tasks: creating files, adding endpoints, scaffolding tests, adding fields, etc.

**Before writing any new file or adding code to an existing file**, run `jig list --skills --claude --json` to check if a recipe already handles it. If one exists, use that skill — it produces correct, consistent output in one shot.

If a skill uses jig and jig fails (non-zero exit), it prints the rendered content to stderr. Apply that output manually instead of starting from scratch.
