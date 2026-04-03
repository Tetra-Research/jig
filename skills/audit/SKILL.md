---
name: audit
description: Audit installed skills and agents against best practices. Use when reviewing skill quality, checking for common problems, or before upgrading existing skills. Read-only — produces a report, changes nothing.
allowed-tools: Read Glob Grep
---

# Audit Skills

Read-only audit of installed skills and agents. Grades each one against `${CLAUDE_SKILL_DIR}/../improve/references/best-practices.md` and produces a structured report. Changes nothing.

## Arguments

```
/audit                    — audit all skills and agents
/audit event-logging      — audit a specific skill by name
/audit .claude/agents/    — audit all agents in a directory
```

If `$ARGUMENTS` names a specific skill, audit only that one. If empty, audit everything.

## Step 1: Discover Skills

Find all SKILL.md files and agent definitions:

```
skills/*/SKILL.md
.claude/skills/*/SKILL.md
.claude/agents/*.md
```

For each file, read the full content. You need the actual text to audit, not just the path.

## Step 2: Grade Each Skill

Read `${CLAUDE_SKILL_DIR}/../improve/references/best-practices.md` for the grading criteria.

For each skill or agent, evaluate these dimensions:

### Description Quality
- Is it a trigger condition or a generic summary?
- Does it front-load words a user would naturally say?
- Would Claude auto-invoke this at the right time?
- **Pass**: "Review code for security vulnerabilities. Use when auditing code or before merging PRs."
- **Fail**: "A helpful skill that reviews code and provides feedback."

### Structure & Size
- Is SKILL.md under 200 lines?
- If over 200 lines, does it use `references/` for progressive disclosure?
- Is content appropriately split between main file and references?

### Gotchas Section
- Does it have one?
- Are the gotchas specific and non-obvious?
- **Fail**: No gotchas section at all
- **Fail**: Vague gotchas like "be careful with edge cases"
- **Pass**: Specific gotchas drawn from real failure modes

### Hardcoded Paths
- Are there file paths, UUIDs, PKs, or magic numbers that will go stale?
- Does it describe *what to look for* or *where to find it*?

### Content Quality
- Does it duplicate CLAUDE.md content?
- Does it state obvious things Claude already knows?
- Is the content specific to the org's patterns, or generic coding advice?

### Side Effect Protection
- Does the skill modify files, query production, or mutate external systems?
- If so, does it have `disable-model-invocation: true`?

### Obvious Instructions
- Are there instructions like "fix any errors" or "run the tests" that add no value?
- Does the skill tell Claude things it would do anyway?

## Step 3: Produce Report

Output a structured report. For each skill:

```
### <skill-name> (<location>)
Lines: <count> | References: <yes/no> | Gotchas: <yes/no>

| Dimension              | Grade | Finding                                    |
|------------------------|-------|--------------------------------------------|
| Description            | ✓/✗   | <specific finding>                         |
| Structure & Size       | ✓/✗   | <specific finding>                         |
| Gotchas                | ✓/✗   | <specific finding>                         |
| Hardcoded Paths        | ✓/✗   | <specific finding>                         |
| Content Quality        | ✓/✗   | <specific finding>                         |
| Side Effect Protection | ✓/✗   | <specific finding or N/A>                  |

**Priority fix**: <the single most impactful improvement for this skill>
```

After all individual reports, add a summary:

```
## Summary

Audited: <count> skills, <count> agents
Passing all checks: <count>
Most common issues:
1. <issue> — <count> skills affected
2. <issue> — <count> skills affected
3. <issue> — <count> skills affected

Recommended upgrade order:
1. <skill> — <why this one first>
2. <skill> — <why>
3. <skill> — <why>
```

The upgrade order should prioritize skills that are heavily used but poorly built — high impact, low quality. If you can't determine usage, prioritize by how many checks failed.

## Gotchas

- **Don't suggest fixes in the audit.** This is a report, not an improvement session. Point to `/upgrade <skill-name>` for fixing.
- **Agents have different rules.** Agents don't have `disable-model-invocation`. They do have `tools` restrictions and `model` selection — audit those instead.
- **Some hardcoded paths are intentional.** Config files, fixture paths, and API endpoints may be correct to hardcode. Flag them but note when they seem intentional vs. fragile.
- **"No gotchas" is always a fail.** Every skill has edge cases. If there's no gotchas section, the author just didn't document them yet.
