# Code Review

Perform a thorough, adversarial code review channeling the rigor of top open source maintainers.

## Usage
```
/review                    # Review current branch vs main
/review <path>             # Review a specific file or directory
```

## Arguments
$ARGUMENTS

---

## Process

### Step 1: Load Context

Read project context to understand conventions:

1. **CLAUDE.md** -- Project conventions and constraints
2. **README.md** -- Project overview
3. Any design docs, specs, or plans relevant to the changes

### Step 2: Identify What Changed

```bash
# Branch diff
git diff main...HEAD --stat
git diff main...HEAD

# Or recent commits
git log --oneline -20
```

Categorize: new files, modified files, deleted files.

### Step 3: Run the Review

Follow the review skill methodology (`.claude/skills/review/SKILL.md`):

1. **Fresh Eyes Pass** -- Quick scan for anything that feels wrong
2. **Correctness** (Linus Mode) -- Logic errors, error handling, edge cases, memory
3. **Design** (Rich Hickey Mode) -- Simplicity, state, interfaces
4. **Consistency** (Yehuda Mode) -- Matches project conventions and existing code
5. **Debuggability** (Bryan Cantrill Mode) -- Error messages, logging, failure modes
6. **Testing** -- Coverage, quality, missing tests
7. **Security** -- Input validation, injection, secrets
8. **Documentation** (Steve Klabnik Mode) -- Public APIs, complex logic
9. **LLM Traps** -- Hallucinated APIs, silent failures, pattern imposition

### Step 4: Write the Review

Output with this structure:

```markdown
## Review: <target>

**Verdict:** [Approve | Request Changes | Needs Discussion]
**Severity:** [Critical | Major | Minor] issues found

### Critical Issues
[Must fix]

### Major Issues
[Should fix]

### Minor Issues
[Nice to fix]

### Questions
[Need clarification]

### What's Good
[Call out strengths]
```

Be specific: file, line number, what's wrong, how to fix.
