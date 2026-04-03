---
name: clean-copy
description: Recreate the current branch with a clean, narrative commit history. Use before creating a PR from a messy development branch, after exploratory coding with WIP commits, or when commit history is hard to review.
allowed-tools: Bash(git*), Bash(gh*), Read, Glob, Grep
disable-model-invocation: true
argument-hint: [clean-branch-name]
---

# Clean Copy

Recreate the current branch with a polished, reviewable commit history. The final state of the clean branch must be identical to the source — only the history changes.

## Steps

### Step 1: Validate the Source Branch

Confirm the branch is ready:

1. Working tree is clean (no uncommitted changes). If dirty, stop and tell the user.
2. Branch is not stale — `origin/master` (or `origin/main`) is an ancestor of HEAD.
3. Record the current branch name as `SOURCE_BRANCH`.

### Step 2: Analyze the Changes

Understand the full scope before planning any commits:

1. Get the commit log from master to HEAD.
2. Get the full diff (stat + patch) from master to HEAD.
3. Identify the logical units of work — not by how they were committed, but by what they accomplish.

### Step 3: Plan the Narrative

Design the commit sequence as if writing a tutorial for a reviewer. Group changes so each commit:

- **Represents one coherent idea** — a schema, a feature, a refactor
- **Is self-contained enough to pass pre-commit hooks** — don't commit a function in one commit and its imports in the next
- **Builds on the previous commit** — a reviewer reading in order should never be confused

Typical ordering:
1. Foundation — types, schemas, models, migrations
2. Implementation — logic in dependency order
3. Tests — alongside or after their implementation
4. Polish — docs, config, minor fixes

Present the planned commit sequence to the user before proceeding. Get confirmation.

### Step 4: Create the Clean Branch

Branch from the latest master/main:

- If `$ARGUMENTS` is provided, use it as the branch name
- Otherwise, name it `${SOURCE_BRANCH}-clean`

### Step 5: Replay Changes

For each planned commit, apply the relevant changes from the source branch and commit.

If a pre-commit hook fails on an intermediate commit:
1. Determine if the commit can be restructured to pass (merge it with related changes, reorder).
2. If the commit genuinely cannot pass hooks due to incomplete state, **ask the user** whether to `--no-verify` for that specific commit. Explain what's failing and why.
3. Never silently skip hooks.

### Step 6: Verify

This is non-negotiable. The clean branch must be identical to the source:

```bash
git diff SOURCE_BRANCH CLEAN_BRANCH
```

If there is any diff, fix it before proceeding. The goal is a different history, not different code.

### Step 7: Create PR and Clean Up

After verification passes:

1. **Push the clean branch** and create a PR. Follow the project's standard PR conventions.
2. **Ask the user** for any details needed for the PR (title, description, reviewers, linked issues).
3. **Check for an existing PR** on the source branch. If one exists, tell the user and ask if they want to close it.
4. **Ask if the source branch should be deleted** (local and remote). The source branch has served its purpose — but let the user decide.

## Gotchas

- **Don't lose changes.** The verification diff in Step 6 is the safety net. Never skip it. If it shows differences, something was missed during replay — go back and fix it before pushing.
- **Commit boundaries vs hook compliance.** The tension is real: clean narrative wants small focused commits, but hooks may need more context to pass. Prefer slightly coarser commits over skipping hooks. Two well-grouped commits beat four that needed `--no-verify`.
- **Don't rewrite someone else's branch** without their knowledge. This creates a new branch — the source branch is untouched — but pushing and PR-ing a rewrite of shared work can surprise collaborators.
- **Binary files and generated content.** These don't cherry-pick or patch cleanly. Apply them as whole-file copies from the source branch.
- **Submodule changes.** If the source branch updated submodules, replay those as a separate commit to keep the diff readable.
