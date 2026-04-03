# Skill Best Practices

Principles for writing effective skills. Use this as a review checklist before finalizing a draft.

---

## Description

The description field determines when Claude auto-invokes the skill. It's a trigger condition, not a summary.

**Good** (front-loads when to use):
```
Review code for security vulnerabilities, performance issues, and style violations. Use when auditing code or before merging PRs.
```

**Bad** (generic summary):
```
A helpful skill that reviews code and provides feedback.
```

- Front-load the words a user would naturally say
- Under 250 characters
- Include 2-3 trigger scenarios

## Structure

### Keep SKILL.md Under 200 Lines

Long skills consume context. Move reference material to separate files in `references/`. Most skills only need SKILL.md — add folders only when the content demands it.

### Progressive Disclosure

Tell Claude what files are in your skill directory. It will read them when needed:
- Detailed signatures → `references/api.md`
- Code examples → `references/examples.md`
- Output templates → `templates/output.md`

## Content

### Don't State the Obvious

Claude knows a lot about coding. Focus on information that pushes Claude out of its normal way of thinking — your org's specific conventions, non-obvious decisions, internal patterns that aren't self-evident from the code.

### Gotchas Are the Highest-Signal Content

Build the gotchas section from real failure modes the user has experienced:
- What did Claude get wrong?
- What edge cases caught people?
- What conventions does Claude naturally violate?

Each gotcha should be specific enough to prevent a real mistake. "Be careful with edge cases" is useless. "The `created_at` field uses UTC but `display_date` uses the hotel's timezone — never compare them directly" is useful.

### Don't Duplicate CLAUDE.md

Claude already has CLAUDE.md loaded. Skills that repeat project conventions waste context and can contradict when CLAUDE.md changes.

### Ground Content in User References

The instructions in a skill should be derived from examples the user considers good — not from Claude's autonomous search of the codebase. Describe *patterns*, not *paths*. A skill that says "follow the authentication pattern in the middleware stack" survives refactors; one that says "copy `src/middleware/auth.py` line 42" breaks silently.

## Behavior

### Avoid Railroading

Give Claude the information it needs, but give it flexibility to adapt. Rigid step-by-step scripts break when the situation doesn't match exactly. Provide the conventions and constraints, let Claude reason about application.

### Handle $ARGUMENTS Both Ways

When auto-invoked by description match, `$ARGUMENTS` is empty. Design skills to work with or without arguments.

### Side Effects Need Protection

Skills that deploy, post to Slack, delete resources, or otherwise affect the outside world should set `disable-model-invocation: true` so only the user can invoke them via `/name`.

## References & Paths

### Don't Hardcode Paths

Code moves. Files get renamed. Instead of pointing to specific files, describe *what to look for*:
- "Find the authentication middleware in the Django middleware stack"
- "Search for the `@require_auth` decorator usage"

### Use Variable References

For files within the skill itself, use `${CLAUDE_SKILL_DIR}` and `${CLAUDE_PLUGIN_ROOT}`. Never hardcode absolute paths.

## Common Mistakes

- Writing a skill when a CLAUDE.md rule would suffice (applies everywhere vs. specific workflow)
- Writing a skill when an agent is needed (tool restrictions, model override)
- Making the skill too long when most content should be in reference files
- Forgetting `disable-model-invocation` for destructive skills
- Writing generic descriptions that never auto-trigger
- Searching the codebase for "good examples" and codifying tech debt
- Hardcoding file paths that will go stale
