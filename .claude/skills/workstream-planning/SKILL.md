---
name: workstream-planning
description: Workstream planning and documentation patterns. Activates when discussing planning, editing PLAN/SPEC/SHARED-CONTEXT, or structuring milestones.
---

# Workstream Planning

## Document Hierarchy

```
docs/workstreams/<name>/
├── discovery/              # Research phase (optional)
├── exec/                   # Execution artifacts (plans, iterations, synthesis)
│   ├── claude-plan-*.md    # Claude planning output
│   ├── codex-plan-*.md     # Codex planning output
│   ├── synthesized.md      # Merged plan (symlink to latest)
│   ├── iteration-*.md      # Execution iterations
│   └── execution-summary-*.md
├── reviews/                # Plan and code review outputs
├── tasks/                  # Per-task ephemeral docs
│   └── <task>/
│       ├── CONTEXT.md      # Task scope and constraints
│       └── VALIDATION.md   # Test traceability matrix
├── PLAN.md                 # Phases, milestones, status (source of truth)
├── SPEC.md                 # Requirements + EARS acceptance criteria
├── SHARED-CONTEXT.md       # Accumulated knowledge
└── NARRATIVE.md            # Human-readable explanation
```

## EARS Requirements Format

All acceptance criteria in SPEC.md use EARS (Easy Approach to Requirements Syntax):

| Type | Pattern |
|------|---------|
| **Ubiquitous** | The system SHALL `<response>` |
| **Event** | WHEN `<trigger>`, the system SHALL `<response>` |
| **State** | WHILE `<state>`, the system SHALL `<response>` |
| **Option** | WHERE `<feature>`, the system SHALL `<response>` |
| **Unwanted** | IF `<condition>`, the system SHALL `<response>` |

Each criterion gets an ID (AC-N.M) and a `Traces To` test reference.

## Planning Workflow

```
ws-init → ws-plan (dual-agent) → ws-plan-review → execute → validate → review → consolidate
```

1. **ws-plan** runs both Claude + Codex against workstream context, saves to `exec/`
2. Human reviews both outputs, synthesizes (or uses `--synthesize`)
3. **ws-plan-review** adversarially checks for gaps, contradictions, EARS compliance
4. **ws-execute** runs iterative implementation with fresh-context retries
5. **validate** checks tests + VALIDATION.md coverage
6. **ws-review** adversarial code review against SPEC
7. **ws-consolidate** captures learnings into durable docs

## Phase Structure

```markdown
### Phase N: <Name>
Status: Planned | In Progress | Complete

#### Milestones
- [ ] N.1: Description
- [ ] N.2: Description (P) ← parallel-safe

#### Validation Criteria
- Testable condition
```

## Document Consistency Rules

- PLAN.md is source of truth for status/phases
- SPEC.md is canonical for requirements and interfaces
- SHARED-CONTEXT references SPEC (doesn't redefine)
- Update PLAN/SPEC/SHARED-CONTEXT/NARRATIVE together on status changes
