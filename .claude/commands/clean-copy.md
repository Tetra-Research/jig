---
description: Reimplement current branch with clean, narrative commit history
allowed-tools: Bash(git*), Bash(gh*)
---

# Clean Copy

Recreate the current branch with a polished, reviewable commit history.

## When to Use

- Before creating a PR from a messy development branch
- After exploratory coding with lots of WIP commits
- When commit history is hard to review

## Workflow

### Phase 1: Validate Source Branch

1. Ensure working tree is clean (no uncommitted changes)
2. Ensure branch is not stale relative to main
3. Record current branch name as `SOURCE_BRANCH`

```bash
# Check for clean state
git status --porcelain
git fetch origin main
git merge-base --is-ancestor origin/main HEAD
```

### Phase 2: Analyze Changes

1. Get full diff from main to understand final state
2. List all commits on the branch
3. Understand the logical changes being made

```bash
git log main..HEAD --oneline
git diff main...HEAD --stat
git diff main...HEAD
```

### Phase 3: Create Clean Branch

Create a new branch from main:

```bash
CLEAN_BRANCH="${SOURCE_BRANCH}-clean"
# Or use $ARGUMENTS if provided
git checkout main
git pull origin main
git checkout -b "$CLEAN_BRANCH"
```

### Phase 4: Plan Narrative

Design commit sequence as if writing a tutorial:

1. **Foundation commits first** - Types, traits, core structures
2. **Implementation commits** - Logic in dependency order
3. **Test commits** - Can be with implementation or separate
4. **Polish commits** - Docs, formatting, minor fixes

Each commit should:
- Represent one coherent idea
- Compile and pass tests (when possible)
- Have a clear, descriptive message

### Phase 5: Reimplementation

For each planned commit:

1. Apply the relevant changes
2. Stage the files
3. Commit with descriptive message

```bash
# Use --no-verify for intermediate commits if hooks fail
# (they may fail due to incomplete state)
git commit --no-verify -m "Add Foo trait with validation"

# Final commit should pass all hooks
git commit -m "Final polish and documentation"
```

### Phase 6: Verification

Confirm clean branch matches source exactly:

```bash
# Should show no diff
git diff "$SOURCE_BRANCH" "$CLEAN_BRANCH"

# Run full validation
cargo check
cargo test
```

### Phase 7: Create PR (Optional)

If requested, create the PR:

```bash
git push -u origin "$CLEAN_BRANCH"
gh pr create --title "..." --body "..."
```

## Example

**Before (messy):**
```
abc1234 wip
def5678 fix tests
ghi9012 more wip
jkl3456 actually fix it
mno7890 oops forgot file
pqr1234 final final
```

**After (clean):**
```
aaa1111 Add RecipeParser with YAML deserialization
bbb2222 Implement template rendering via minijinja
ccc3333 Add inject and replace file operations
ddd4444 Add comprehensive test coverage
```

## Notes

- Do NOT include `Co-Authored-By` in clean commits
- The goal is a history that tells a story
- Each commit should be reviewable in isolation
- Final state MUST match source branch exactly
