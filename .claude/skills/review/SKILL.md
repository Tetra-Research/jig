---
name: review
description: |
  Adversarial code review channeling top OSS maintainers.
  Activates when:
  - User mentions "review", "code review", "check the code", "look over"
  - Before merging a PR or branch
  - Asking for implementation feedback
  - User wants a thorough critique of code changes
---

# Code Review

Perform a thorough code review channeling the rigor of top open source maintainers. This is not a rubber stamp -- it's a genuine adversarial review looking for bugs, design issues, and gaps.

## Philosophy

Channel the personalities of legendary code reviewers:

**Linus Torvalds** (Linux) -- "Does this need to exist? Is it correct? Is the interface clean?"
- Zero tolerance for unnecessary complexity
- Memory safety and error handling must be bulletproof
- Every line must justify its existence

**Rich Hickey** (Clojure) -- "Simple vs Easy. What problem does this actually solve?"
- Distinguish essential complexity from accidental complexity
- State management scrutiny
- Question abstractions -- are they earning their keep?

**Bryan Cantrill** (illumos/DTrace) -- "Can I debug this at 3am in production?"
- Error messages must be actionable
- Failure modes must be explicit
- Observability is not optional

**Yehuda Katz** (Rails/Ember/Rust) -- "How does this feel to use?"
- API ergonomics matter
- Consistency with ecosystem conventions
- Progressive disclosure of complexity

**Steve Klabnik** (Rust docs) -- "Can someone else understand this?"
- Documentation that teaches, not just describes
- Examples that actually work
- Code that reads like prose

---

## Review Process

### Step 1: Load Context

Understand what you're reviewing:

1. **Project docs** -- Read CLAUDE.md, README, any architecture/design docs
2. **Spec or plan** -- If there's a spec or plan for this work, read it
3. **Existing patterns** -- Scan the codebase for conventions this code should follow

### Step 2: Identify What Changed

Determine the scope of review:

```bash
# If reviewing a branch
git diff main...<branch> --stat
git diff main...<branch>

# If reviewing recent work
git log --oneline -20
git diff HEAD~N..HEAD
```

List all files changed and categorize:
- New files (need full review)
- Modified files (need contextual review)
- Deleted files (verify nothing broken)

### Step 3: The "Fresh Eyes" Pass

Before diving deep, do a quick scan with fresh eyes:

> "Read over all the new code and modified code looking super carefully for any obvious bugs, errors, problems, issues, confusion. Note anything that feels wrong."

This catches:
- Obvious typos and copy-paste errors
- Logic that doesn't make sense on first read
- Missing error handling visible at a glance
- Inconsistent naming or style
- Code that makes you go "wait, what?"

**Document everything that feels off, even if you're not sure why.**

### Step 4: Correctness Review (Linus Mode)

For each changed file, examine:

**Logic Errors:**
- Off-by-one errors
- Incorrect boolean logic
- Wrong operator (`=` vs `==`, `&&` vs `||`)
- Integer overflow/underflow
- Null/None dereference possibilities

**Error Handling:**
- Are all `Result`/`Option` types handled?
- Do error messages include context (what failed, why, what to do)?
- Are errors propagated correctly with `?`?
- Any panics that should be Results?

**Edge Cases:**
- Empty collections
- Zero/negative values
- Maximum values
- Unicode and special characters
- Concurrent access

**Memory & Resources:**
- Ownership correct?
- Lifetimes make sense?
- Resources cleaned up? (files, connections, locks)
- Any unbounded growth? (vectors, hashmaps without limits)

### Step 5: Design Review (Rich Hickey Mode)

Question every abstraction:

**Simplicity:**
- Could this be simpler? What would you delete?
- Is complexity essential or accidental?
- Are there abstractions that aren't earning their keep?
- Is this solving the actual problem or an imagined generalization?

**State:**
- Where does state live?
- Can state get out of sync?
- Is mutability necessary?
- Are invariants enforced at the type level?

**Interfaces:**
- Is the API obvious to use correctly?
- Is it hard to use incorrectly?
- Does it follow the principle of least surprise?
- Are there too many parameters? (>3 is suspicious)

### Step 6: Consistency Review (Yehuda Mode)

Check alignment with codebase patterns:

**Against project conventions:**
- [ ] Error handling matches project conventions
- [ ] Type patterns match (newtypes, etc.)
- [ ] Async patterns match
- [ ] Testing patterns match

**Against existing code:**
- Does naming match existing conventions?
- Does structure match similar modules?
- Are similar problems solved the same way?

**Against spec (if one exists):**
- Does implementation match requirements?
- Are all acceptance criteria addressed?
- Any requirements missed or misinterpreted?

### Step 7: Debuggability Review (Bryan Cantrill Mode)

Imagine debugging this at 3am:

**Error Messages:**
- Do errors say what went wrong?
- Do they say where it went wrong?
- Do they suggest what to do about it?
- Can you grep for the error message?

**Logging/Tracing:**
- Are important operations logged?
- Is there enough context in logs?
- Are log levels appropriate?
- Can you trace a request through the system?

**Failure Modes:**
- What happens when dependencies fail?
- Are timeouts configured?
- Is there retry logic? Should there be?
- What's the blast radius of a failure?

### Step 8: Testing Review

Examine test coverage and quality:

**Coverage:**
- Are the happy paths tested?
- Are error paths tested?
- Are edge cases tested?

**Quality:**
- Do tests test behavior, not implementation?
- Are tests readable and maintainable?
- Do test names describe what's being tested?
- Are there any flaky test patterns?

**Missing Tests:**
- What scenarios aren't tested?
- What would break that tests wouldn't catch?

### Step 9: Security Review

Check for common vulnerabilities:

- [ ] Input validation at system boundaries
- [ ] No SQL injection (parameterized queries)
- [ ] No command injection (shell escaping)
- [ ] Secrets not logged or exposed in errors
- [ ] Authentication/authorization checked
- [ ] No path traversal vulnerabilities
- [ ] Sensitive data encrypted at rest/transit

### Step 10: Documentation Review (Steve Klabnik Mode)

**Code Documentation:**
- Are public APIs documented?
- Do complex algorithms have explanations?
- Are non-obvious decisions explained with comments?
- Are there examples in doc comments?

**External Documentation:**
- Does README need updating?
- Are breaking changes documented?

### Step 11: LLM-Generated Code Traps

Code written by Claude or other LLMs has specific failure patterns. Watch for these:

**Hallucinated APIs:**
- Functions/methods that don't exist
- Wrong function signatures or argument names
- Invented crate features or trait methods
- APIs that look right but aren't real -- verify against actual docs

**Silent Failures:**
- Code that runs but doesn't do what's intended
- Removed safety checks to make code "work"
- Error handling that swallows errors silently

**Confidence Without Calibration:**
- Equally confident tone whether correct or guessing
- No hedging on uncertain areas

**Pattern Imposition:**
- Modern patterns in legacy codebases
- New dependencies when existing ones work
- Refactoring mixed with bug fixes
- "Improvements" beyond what was asked

**Trial-and-Error Residue:**
- Multiple attempts visible in git history
- Commented-out failed approaches
- Inconsistent solutions in different parts of code
- Over-complicated solutions from iterative "fixing"

**Rust-Specific LLM Issues:**
- Lifetime annotations that compile but are wrong semantically
- Unnecessary `.clone()` to silence borrow checker
- `unwrap()` where `?` should be used
- Misuse of `Arc<Mutex<>>` when simpler patterns work
- Wrong trait bounds (compiles but won't work with real types)

**Temporal Confusion:**
- Outdated API usage from training data
- Deprecated patterns presented as current
- Version mismatches (assumes older/newer crate versions)

**The "Looks Right" Test:**
If code looks suspiciously clean and idiomatic on first read, be MORE skeptical, not less. LLMs excel at producing code that looks correct. Verify:
- Does each function actually exist?
- Are the argument types actually correct?
- Does the logic actually do what the comments say?

---

## Output Format

Structure your review as:

```markdown
## Review: <target>

**Reviewer Mindset:** [Which personalities were most relevant]
**Verdict:** [Approve | Request Changes | Needs Discussion]
**Severity:** [Critical | Major | Minor] issues found

---

### Fresh Eyes Findings
[Anything that felt off on first read]

### Critical Issues
[Must fix before merge -- bugs, security, data loss risks]

### Major Issues
[Should fix -- design problems, missing error handling, test gaps]

### Minor Issues
[Nice to fix -- style, naming, documentation]

### Questions
[Things that need clarification or discussion]

### What's Good
[Explicitly call out well-done aspects]

---

### Checklist

**Correctness:**
- [ ] Logic is correct
- [ ] Error handling is complete
- [ ] Edge cases handled

**Design:**
- [ ] Abstractions earn their complexity
- [ ] State management is sound
- [ ] API is hard to misuse

**Consistency:**
- [ ] Matches project conventions
- [ ] Matches codebase patterns

**Debuggability:**
- [ ] Error messages are actionable
- [ ] Logging is sufficient
- [ ] Failure modes are explicit

**Testing:**
- [ ] Happy paths covered
- [ ] Error paths covered
- [ ] Edge cases covered

**Security:**
- [ ] Input validation present
- [ ] No injection vulnerabilities
- [ ] Secrets protected

**Documentation:**
- [ ] Public APIs documented
- [ ] Complex logic explained

**LLM Traps:**
- [ ] All APIs verified to exist
- [ ] No hallucinated function signatures
- [ ] No unnecessary .clone() or unwrap()
- [ ] No silent error swallowing
- [ ] No pattern imposition beyond requirements
- [ ] Logic matches comments/docs
```

---

## Notes

- This is an adversarial review -- actively look for problems
- Be specific: file, line number, what's wrong, how to fix
- Praise what's done well -- good code deserves recognition
- If you'd reject this in a top open source project, say so
- The goal is quality, not speed
