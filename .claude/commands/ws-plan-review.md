# Workstream Plan Review

Adversarial review of planning documents for a workstream.

## Usage
```
/ws-plan-review <workstream-name>
```

## Arguments
$ARGUMENTS

---

## IMPORTANT: Read-Only Review

This is a review, not implementation. Do not modify source files.

## Review Process

### 1. Load Context

Read all workstream docs:
- `docs/workstreams/<name>/PLAN.md`
- `docs/workstreams/<name>/SPEC.md`
- `docs/workstreams/<name>/SHARED-CONTEXT.md`
- `docs/workstreams/<name>/NARRATIVE.md`
- `docs/workstreams/<name>/exec/synthesized.md` (if exists)
- `docs/INVARIANTS.md`
- `docs/ARCHITECTURE.md`

### 2. Consistency Check

Build a map across all docs. Hunt for:
- **Contradictions** between SPEC and PLAN
- **Missing requirements** — anything in PLAN not covered by SPEC
- **Orphaned criteria** — EARS acceptance criteria with no test mapping
- **Scope gaps** — what's not specified that should be

### 3. EARS Format Audit

For every acceptance criterion in SPEC.md:
- [ ] Is it in EARS format (WHEN/WHILE/IF/WHERE/SHALL)?
- [ ] Does it have an AC-N.M identifier?
- [ ] Does the `Traces To` column reference a test?
- [ ] Is the criterion testable (not vague)?

Flag any free-form acceptance criteria that should be converted to EARS.

### 4. Completeness Check

- [ ] All phases in PLAN.md have validation criteria
- [ ] All FR/NFR in SPEC.md have EARS acceptance criteria
- [ ] Data model covers all entities referenced in requirements
- [ ] Error handling is specified for failure modes
- [ ] Dependencies between workstreams are documented

### 5. Output

Structure findings as:

**Critical** (blocks implementation):
- Finding with `file:line` reference

**Major** (should fix before implementation):
- Finding with `file:line` reference

**Minor** (nice to fix):
- Finding with `file:line` reference

Save review to `docs/workstreams/<name>/reviews/<timestamp>.md`
