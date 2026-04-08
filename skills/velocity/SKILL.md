---
name: velocity
description: Systematic DX improvement hunt — find friction, inconsistency, and missing abstractions across PRs, code, or tickets, then feed findings into /improve. Use when looking for ways to accelerate development or compound Claude Code's utility.
allowed-tools: Read Write Edit Glob Grep Bash Agent WebFetch AskUserQuestion
disable-model-invocation: true
argument-hint: [scope — e.g. "my PRs", "backend/module/component", "<ticket-id>"]
---

# Velocity Hunt

Systematically find DX friction, inconsistency, and missing abstractions — then turn findings into Claude Code extensions via `/improve`.

## Step 1: Scope the Hunt

Before investigating anything, interview the user to define a clear scope. Do not start exploring until you have agreement on what to look at.

Ask: **"What do you want me to look at?"**

Offer concrete options based on what's available:

- **Your recent PRs** — `gh pr list --author=@me --state=merged --limit=20`
- **A specific PR** — user provides a URL or number
- **Recent team PRs** — `gh pr list --state=merged --limit=30`
- **A codebase area** — user points at a directory or domain
- **CODEOWNERS domain** — pick a team/area from CODEOWNERS and cover what they own
- **Linear tickets** — look at recent tickets for a team or label for recurring pain

If `$ARGUMENTS` is provided, use it to seed the conversation — but still confirm scope before investigating.

### If the user pushes back on scoping

Explain why: *"Without a scope, I'll either look at everything superficially or go deep on the wrong thing. A velocity hunt works best when it's focused — we can always run another one on a different area. What's been causing you the most friction lately?"*

### Ask about themes

Once you have the scope, ask: **"Are there any themes you care about most within that area?"** Examples:

- Error handling consistency
- Testing patterns
- API endpoint structure
- Missing documentation or conventions
- Onboarding friction for new developers
- Repeated manual workflows

If they have themes, focus the investigation there. If not, explore broadly within the scope.

### Calibrate depth

Ask how deep they want to go:
- **Quick scan** — skim for obvious patterns (10-15 min of investigation)
- **Thorough hunt** — read code, trace patterns across files, check for consistency (30+ min)

## Step 2: Investigate

**Use subagents aggressively.** Every distinct theme or codebase area should be explored by its own subagent via the Agent tool. This preserves the main session's context window for synthesis and the user conversation. Launch subagents in parallel whenever the areas are independent.

Examples:
- Scope is "recent PRs" with 3 themes → 3 parallel subagents, one per theme
- Scope is a codebase area → subagent per subdirectory or per concern (patterns, consistency, tooling gaps)
- Scope is CODEOWNERS domain → subagent per owned path

Each subagent should return a structured summary: what it found, where (file:line), frequency, and a one-sentence friction assessment. You synthesize and rank the results in the main session.

You're looking for:

- **Repeated patterns** — same boilerplate appearing 3+ times that could be a skill, generator, or abstraction
- **Inconsistency** — the same thing done 3 different ways across the codebase
- **Missing tooling** — workflows where Claude (or the developer) has to do manual steps that could be automated
- **Convention drift** — established patterns that newer code isn't following
- **Friction points** — things that are tedious, error-prone, or require tribal knowledge
- **Skills/rules gaps** — places where a Claude Code extension would prevent recurring mistakes

Read `${CLAUDE_SKILL_DIR}/references/evaluation.md` for heuristics on evaluating what you find.

### What you're NOT looking for

- Bugs (that's triage)
- Performance problems (that's profiling)
- One-off code smells (not worth systematizing)
- Patterns that have appeared only 1-2 times (wait for the third)

## Step 3: Present Findings

Present each finding in conversation with this structure:

**Finding: [title]**
- **Pattern**: What you observed (with file/line references)
- **Frequency**: How often this appears / how many instances
- **Friction**: What this costs in time, consistency, or correctness
- **Extension type**: What kind of Claude Code extension would fix this (skill, rule, agent, hook, CLAUDE.md)
- **Effort**: Low / Medium / High to implement the extension

Rank findings by impact. Lead with the most valuable ones.

## Step 4: Handoff to /improve

This is where findings become real. The approach depends on how many actionable findings there are.

### Single finding

Offer to feed it directly into `/improve` in the current session. You already have the context — the problem statement, code references, and frequency data. Transition naturally: present the finding, ask if they want to act on it, then shift into the improve workflow.

### Multiple findings (2+)

Recommend this workflow:

1. **Write a temporary findings doc** — create `.claude/velocity-findings.md` with all findings and a pre-built `/improve` prompt for each one. Format each prompt so the user can copy-paste it into a new Claude session:

```markdown
## Finding: [title]
[problem statement, references, frequency data]

### Improve prompt
> /improve [concise description of the problem, what extension type to create, and which files/PRs to use as references]
```

2. **Create a branch** — `git checkout -b velocity/YYYY-MM-DD`
3. **Work through findings** — the user runs `/improve` for each finding (either in this session or new ones), creating extensions one at a time
4. **PR description** — when they're ready to merge, include the findings summary in the PR description so reviewers understand the motivation
5. **Clean up** — delete `.claude/velocity-findings.md` before merging. The findings doc is scaffolding, not a permanent artifact. The extensions themselves are the durable output.

### Always offer both paths

Some users will want to knock out one finding and come back later. Others will want to batch. Let them choose.

## Gotchas

- **Don't codify tech debt.** You're investigating the codebase to find friction, not to find "good examples." When you spot a pattern, evaluate whether it's a good pattern worth systematizing or a bad pattern worth replacing. If in doubt, ask the user.
- **3+ occurrences is the threshold.** If a pattern has appeared once or twice, it's not worth abstracting yet. Note it and move on. The cost of premature abstraction is higher than the cost of a little duplication.
- **Velocity is not refactoring.** The output is Claude Code extensions (skills, rules, hooks, agents), not code changes. If a finding suggests "refactor this module," reframe it: "What Claude Code extension would prevent this pattern from recurring or make the refactor easier?"
- **The findings doc is temporary.** Never let it survive past the PR. The extensions are the permanent output; the findings doc is just a work queue.
- **Scope creep kills hunts.** If the investigation surface area keeps expanding, stop and re-scope with the user. A focused hunt that produces 3 actionable findings beats a broad survey that produces 15 vague observations.
