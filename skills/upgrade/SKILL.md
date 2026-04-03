---
name: upgrade
description: Upgrade an existing skill or agent by auditing it, gathering user references, and rewriting it with real content. Use when a skill is thin, sloppy, missing gotchas, or needs improvement. Different from /improve which creates new extensions.
allowed-tools: Read Write Edit Glob Grep Bash AskUserQuestion
---

# Upgrade Skill

Upgrade an existing skill or agent. Starts with an audit to identify problems, then walks the user through fixing them with the same reference-gathering interview as `/improve` — because the same problem applies: thin skills exist because the author didn't provide examples of what good looks like.

This is different from `/improve`:
- `/improve` creates **new** extensions from scratch
- `/upgrade` makes **existing** extensions better

## Arguments

```
/upgrade event-logging        — upgrade a specific skill
/upgrade bridge-expert        — upgrade a specific agent
```

If `$ARGUMENTS` is empty, ask which skill or agent to upgrade.

## Step 1: Audit

Run the audit process from `${CLAUDE_SKILL_DIR}/../audit/SKILL.md` on the target skill. Present the findings to the user — this shows them exactly what's wrong before any changes.

If the skill passes all checks, tell the user and ask if they still want to proceed. Sometimes a passing skill still has room for improvement that the checklist doesn't catch.

## Step 2: Gather References

The same interview pattern as `/improve`. The audit tells you *what's wrong*; references tell you *what good looks like*.

### Ask for good examples

> "The audit found [specific issues]. To fix these, I need to understand what good looks like for this workflow. Point me to 2-3 examples of [this thing] done well — files, PRs, whatever you consider a good reference."

Read every reference they provide. Extract:
- **Conventions** — naming, structure, ordering, style choices
- **Non-obvious decisions** — things that aren't self-evident from the code
- **Implicit rules** — patterns the user follows but hasn't written down
- **Edge cases** — scenarios the references handle that a naive approach would miss

### Ask for failure modes

> "What does Claude get wrong when using this skill today? Or when doing this workflow without the skill?"

This feeds the gotchas section — the thing most existing skills are missing entirely.

### Do not search autonomously for examples

Same rule as `/improve`: the codebase has tech debt. The user curates what "good" looks like. You may search for structural information but never for quality references.

## Step 3: Plan the Rewrite

Present a plan before changing anything:

1. What you're keeping from the existing skill (things that work)
2. What you're changing and why (tied to audit findings)
3. What you're adding (grounded in user references)
4. New file structure if adding `references/` for progressive disclosure

Let the user confirm or adjust.

## Step 4: Rewrite

Rewrite the skill grounded in the user's references:
- Patterns from good examples → core instructions
- Failure modes → gotchas section
- Conventions → rules Claude should enforce
- Audit findings → structural fixes (size, description, progressive disclosure)

### Rewriting guidelines

1. **Preserve what works.** Don't rewrite from scratch if 80% is fine. Fix the broken parts.
2. **Don't hardcode paths from references.** Describe *what to look for*.
3. **Extract long content to references/.** If the skill is over 200 lines, split reference material out.
4. **Rewrite the description as a trigger.** Front-load words the user would say.
5. **Add real gotchas.** Drawn from the failure modes the user described, not generic advice.

## Step 5: Review Against Best Practices

Check the rewrite against `${CLAUDE_SKILL_DIR}/../improve/references/best-practices.md`:

- [ ] Description is a trigger condition, not a summary
- [ ] SKILL.md is under 200 lines; reference material in separate files
- [ ] Gotchas are specific and non-obvious
- [ ] Instructions describe patterns, not hardcoded paths
- [ ] Doesn't duplicate CLAUDE.md content
- [ ] Claude has flexibility to adapt

Present any remaining issues and fix before writing.

## Step 6: Verify

After writing:

1. Read back the modified files to verify correctness
2. Show a before/after comparison of the key changes
3. Suggest: "Run the skill, see where Claude goes wrong, add that to the gotchas section"

## Gotchas

- **Don't delete content the user didn't flag.** The audit might flag something that the user considers intentional. Check before removing.
- **Gotchas need real experience to write well.** If the user can't articulate failure modes, the gotchas section will be thin. That's okay — suggest they revisit after using the skill a few more times.
- **Progressive disclosure isn't always better.** A 150-line SKILL.md with no references is fine. Don't split into references just because you can.
- **Some skills are intentionally simple.** `ask-user-questions-more` is 44 lines and might be exactly right for what it does. Don't over-engineer during upgrade.
