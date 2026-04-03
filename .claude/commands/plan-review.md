# Plan Review

Generate a thorough, adversarial review of planning documents. The goal is to surface contradictions, missing decisions, unclear defaults, and gaps before implementation starts.

## Usage
```
/plan-review <path-to-plan-or-spec>
/plan-review docs/              # Review all docs in a directory
```

## Arguments
$ARGUMENTS

---

## Scope

Read-only. Do not edit code. Only read and critique planning/design documents.

Read whatever planning documents exist:
- PLAN.md, SPEC.md, design docs, architecture docs
- README.md for stated goals and scope
- Any referenced specs (jig.md, etc.)

If key docs are missing, call that out explicitly as a review issue.

---

## Review Method

### Step 1: Build a Consistency Map (Single Source of Truth)

Create a scratch summary of the following from each document:
- **Decisions** (key choices, rationale)
- **Public API surface** (CLI interface, commands, flags, output formats)
- **Data model** (types, schemas, formats)
- **Dependencies/feature flags**
- **Validation criteria and tests**
- **Risk/open questions**

Then check for cross-doc mismatches.

### Step 2: Contradiction Hunt (Must be exhaustive)

Look for conflicts in:
- Chosen libraries/approaches
- Names of config fields and defaults
- Public API promises (commands, flags, return types, errors)
- Error types and variants
- Default behavior vs examples (what happens with defaults?)
- Terminology used inconsistently across docs

### Step 3: Completeness Check

Ensure the plan answers:
- What is in-scope vs out-of-scope (with boundaries)
- Explicit defaults and fallback behavior
- Failure modes and error handling strategy
- Required data model definitions
- Required acceptance criteria
- Dependencies and sequencing
- Risks and open questions

**Type Audit (for each type in any data model):**
For every struct/enum/type referenced, verify:
- [ ] Fully defined with all fields, types, defaults
- [ ] Validation rules documented
- [ ] Example value shown

Flag incomplete types:
> **Incomplete Type:** `Position` referenced in PLAN.md but not fully defined anywhere.

### Step 4: Testability & Validation Trace

Build a mini trace:
```
Requirement -> How to validate -> How to test
```
Call out requirements that lack validation or testing strategy.

### Step 5: Execution Readiness

Verify:
- Plan phases are sequenced correctly
- Dependencies between phases are explicit
- Validation steps are realistic and sufficient
- Scope is achievable

---

## Output Format

Use this structure, and always include file/line references:

```markdown
## Plan Review: <target>

**Verdict:** [Approve | Request Changes | Needs Discussion]

### Critical Issues
[Must fix before implementation: contradictions, missing decisions, unsafe defaults]

### Major Issues
[Should fix: gaps in requirements, missing validation, inconsistent naming]

### Minor Issues
[Nice to fix: wording, clarity, polish]

### Questions
[Clarifications needed]

### What's Solid
[Call out strengths]

### Consistency Map (Short)
| Topic | Source of Truth | Conflicts |
|-------|----------------|-----------|
| CLI interface | jig.md | README says X, spec says Y |
```

Be explicit about impact: "This will cause X to be implemented wrong," not just "inconsistent."

---

## Quality Bar (Non-Negotiable)

This review should be as rigorous as a top OSS maintainer's review:
- No hand-waving
- No "seems fine"
- Every finding grounded in a concrete mismatch or omission
- Specific references with file and line numbers
