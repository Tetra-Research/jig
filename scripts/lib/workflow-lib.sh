#!/bin/bash
# workflow-lib.sh — Template generation, prompt builders, and workflow helpers
# Shared library for all ws-* scripts.

set -euo pipefail

# --- Path Helpers ---

get_repo_root() {
    git rev-parse --show-toplevel 2>/dev/null || echo "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
}

get_docs_dir() {
    echo "$(get_repo_root)/docs"
}

get_workstream_dir() {
    local ws_name="$1"
    echo "$(get_docs_dir)/workstreams/$ws_name"
}

get_task_dir() {
    local ws_name="$1"
    local task_name="$2"
    echo "$(get_workstream_dir "$ws_name")/tasks/$task_name"
}

get_exec_dir() {
    local ws_name="$1"
    local exec_dir
    exec_dir="$(get_workstream_dir "$ws_name")/exec"
    mkdir -p "$exec_dir"
    echo "$exec_dir"
}

# --- Logging ---

log_info() { echo "[INFO] $*" >&2; }
log_warn() { echo "[WARN] $*" >&2; }
log_error() { echo "[ERROR] $*" >&2; }

# --- Template Generation ---

# Generate PLAN.md for a workstream
generate_plan_md() {
    local ws_name="$1"
    local ws_dir="$2"
    local today
    today=$(date +%Y-%m-%d)

    cat > "$ws_dir/PLAN.md" << 'PLANEOF'
# PLAN.md

> Workstream: WSNAME
> Last updated: TODAY
> Status: Initialized

## Objective

<!-- What this workstream accomplishes and why -->

## Phases

### Phase 1: <!-- Phase name -->
Status: Planned

#### Milestones
- [ ] 1.1: <!-- Milestone description -->
- [ ] 1.2: <!-- Milestone description -->

#### Validation Criteria
- <!-- How we know this phase is complete -->

## Dependencies

- **Depends on:** <!-- Other workstreams or none -->
- **Blocks:** <!-- What depends on this -->

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| <!-- decision --> | <!-- choice --> | <!-- why --> |

## Risks / Open Questions

- <!-- Risk or open question -->
PLANEOF

    sed -i '' "s/WSNAME/$ws_name/g; s/TODAY/$today/g" "$ws_dir/PLAN.md"
}

# Generate SPEC.md for a workstream (with EARS-format acceptance criteria)
generate_spec_md() {
    local ws_name="$1"
    local ws_dir="$2"
    local today
    today=$(date +%Y-%m-%d)

    cat > "$ws_dir/SPEC.md" << 'SPECEOF'
# SPEC.md

> Workstream: WSNAME
> Last updated: TODAY

## Overview

<!-- High-level description of what this workstream builds -->

## Requirements

### Functional Requirements

#### FR-1: <!-- Requirement name -->

<!-- Detailed description -->

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-1.1 | Event | WHEN <trigger>, the system SHALL <response> | TEST-1.1 |
| AC-1.2 | State | WHILE <state>, the system SHALL <response> | TEST-1.2 |
| AC-1.3 | Unwanted | IF <condition>, the system SHALL <response> | TEST-1.3 |

<!-- EARS Pattern Reference:
  Ubiquitous:  The system SHALL <response>
  Event:       WHEN <trigger>, the system SHALL <response>
  State:       WHILE <state>, the system SHALL <response>
  Option:      WHERE <feature>, the system SHALL <response>
  Unwanted:    IF <condition>, the system SHALL <response>
-->

#### FR-2: <!-- Requirement name -->

<!-- Detailed description -->

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-2.1 | <!-- type --> | <!-- criterion --> | TEST-2.1 |

### Non-Functional Requirements

#### NFR-1: <!-- Requirement name -->

<!-- Performance, security, reliability requirements -->

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N1.1 | <!-- type --> | <!-- criterion --> | TEST-N1.1 |

## Interfaces

### Public API

```
// Define public interfaces here
```

### Internal Interfaces

```
// Define internal interfaces here
```

## Component Relationships

```mermaid
graph TD
    A[Component] --> B[Dependency]
```

## Data Model

<!-- Describe key data structures -->

## Error Handling

<!-- Define error types and handling strategy -->

## Testing Strategy

- **Spec tests:** Derived from EARS acceptance criteria above
- **Invariant tests:** From INVARIANTS.md constraints
- **Contract tests:** From interface definitions above

## Requirement Traceability

| Requirement | Criteria | Test | Status |
|-------------|----------|------|--------|
| FR-1 | AC-1.1, AC-1.2, AC-1.3 | spec::fr1::* | PENDING |
| FR-2 | AC-2.1 | spec::fr2::* | PENDING |
SPECEOF

    sed -i '' "s/WSNAME/$ws_name/g; s/TODAY/$today/g" "$ws_dir/SPEC.md"
}

# Generate SHARED-CONTEXT.md for a workstream
generate_shared_context_md() {
    local ws_name="$1"
    local ws_dir="$2"
    local today
    today=$(date +%Y-%m-%d)

    cat > "$ws_dir/SHARED-CONTEXT.md" << 'SCEOF'
# SHARED-CONTEXT.md

> Workstream: WSNAME
> Last updated: TODAY

## Purpose

<!-- Brief description of what this workstream does -->

## Current State

- Initialized (TODAY)

## Decisions Made

<!-- Decisions from planning and implementation, with rationale -->

## Patterns Established

<!-- Recurring patterns discovered during implementation -->

## Known Issues / Tech Debt

<!-- Issues to track, not block on -->

## File Ownership

<!-- Which files/modules this workstream owns -->
SCEOF

    sed -i '' "s/WSNAME/$ws_name/g; s/TODAY/$today/g" "$ws_dir/SHARED-CONTEXT.md"
}

# Generate NARRATIVE.md for a workstream
generate_narrative_md() {
    local ws_name="$1"
    local ws_dir="$2"
    local today
    today=$(date +%Y-%m-%d)

    cat > "$ws_dir/NARRATIVE.md" << 'NAREOF'
# NARRATIVE.md

> Workstream: WSNAME
> Last updated: TODAY

## What This Does

<!-- Human-readable explanation of what the workstream builds -->

## Why It Exists

<!-- The problem being solved and why it matters -->

## How It Works

<!-- High-level explanation of the approach -->

```mermaid
graph LR
    Input --> Process --> Output
```

## Key Design Decisions

<!-- Explain the "why" behind major choices -->
NAREOF

    sed -i '' "s/WSNAME/$ws_name/g; s/TODAY/$today/g" "$ws_dir/NARRATIVE.md"
}

# Generate VALIDATION.md for a task (extracts EARS criteria from SPEC.md)
generate_task_validation_md() {
    local ws_name="$1"
    local task_name="$2"
    local task_dir="$3"
    local ws_dir="${4:-}"
    local today
    today=$(date +%Y-%m-%d)

    # Try to extract EARS acceptance criteria from SPEC.md
    local ac_table=""
    local ac_count=0
    if [[ -n "$ws_dir" && -f "$ws_dir/SPEC.md" ]]; then
        # Extract AC- lines from EARS tables
        ac_table=$(grep -E "^\| *AC-" "$ws_dir/SPEC.md" 2>/dev/null | head -30 || true)
        ac_count=$(echo "$ac_table" | grep -c "AC-" 2>/dev/null || echo "0")
    fi

    # Build requirements table from EARS criteria
    local req_table=""
    if [[ -n "$ac_table" && "$ac_count" -gt 0 ]]; then
        req_table=$(echo "$ac_table" | awk -F'|' '{
            gsub(/^[ \t]+|[ \t]+$/, "", $2);  # ID
            gsub(/^[ \t]+|[ \t]+$/, "", $3);  # Type
            gsub(/^[ \t]+|[ \t]+$/, "", $4);  # Criterion
            print "| " $2 " | " $3 " | SPEC.md | `spec::...` | PENDING |"
        }')
    else
        req_table="| <!-- AC-N.N --> | <!-- type --> | SPEC.md#... | \`spec::...\` | PENDING |"
    fi

    cat > "$task_dir/VALIDATION.md" << EOF
# VALIDATION.md

> Workstream: $ws_name
> Task: $task_name
> Last verified: $today

## Spec Requirements -> Tests

| Criterion | EARS Type | Spec Section | Test | Status |
|-----------|-----------|--------------|------|--------|
$req_table

## Invariants -> Tests

| Invariant | Source | Test | Status |
|-----------|--------|------|--------|
| <!-- inv --> | INVARIANTS.md#... | \`invariants::...\` | PENDING |

## Coverage Summary

- Spec criteria: 0/${ac_count} covered
- Invariants: 0/0 covered

## Gaps

All criteria need test implementations.
EOF
}

# --- Prompt Builders (Track B) ---

# Build a planning prompt from workstream context
generate_dual_plan_prompt() {
    local ws_name="$1"
    local ws_dir
    ws_dir="$(get_workstream_dir "$ws_name")"
    local docs_dir
    docs_dir="$(get_docs_dir)"

    local prompt="You are planning the implementation of workstream '$ws_name'.\n\n"

    # Load project-level context
    if [[ -f "$docs_dir/INVARIANTS.md" ]]; then
        prompt+="## Project Invariants\n\n$(cat "$docs_dir/INVARIANTS.md")\n\n"
    fi
    if [[ -f "$docs_dir/ARCHITECTURE.md" ]]; then
        prompt+="## Architecture\n\n$(cat "$docs_dir/ARCHITECTURE.md")\n\n"
    fi

    # Load workstream context
    if [[ -f "$ws_dir/SPEC.md" ]]; then
        prompt+="## Specification\n\n$(cat "$ws_dir/SPEC.md")\n\n"
    fi
    if [[ -f "$ws_dir/PLAN.md" ]]; then
        prompt+="## Current Plan\n\n$(cat "$ws_dir/PLAN.md")\n\n"
    fi
    if [[ -f "$ws_dir/SHARED-CONTEXT.md" ]]; then
        prompt+="## Shared Context\n\n$(cat "$ws_dir/SHARED-CONTEXT.md")\n\n"
    fi

    # Load discovery docs if they exist
    if [[ -d "$ws_dir/discovery" ]]; then
        for doc in "$ws_dir/discovery"/*.md; do
            [[ -f "$doc" ]] && prompt+="## Discovery: $(basename "$doc")\n\n$(cat "$doc")\n\n"
        done
    fi

    prompt+="## Instructions\n\n"
    prompt+="Based on the above context, produce a detailed execution plan.\n\n"
    prompt+="For each phase/milestone:\n"
    prompt+="1. Break down into concrete tasks with file paths\n"
    prompt+="2. Write acceptance criteria in EARS format:\n"
    prompt+="   - Ubiquitous: The system SHALL <response>\n"
    prompt+="   - Event: WHEN <trigger>, the system SHALL <response>\n"
    prompt+="   - State: WHILE <state>, the system SHALL <response>\n"
    prompt+="   - Option: WHERE <feature>, the system SHALL <response>\n"
    prompt+="   - Unwanted: IF <condition>, the system SHALL <response>\n"
    prompt+="3. Identify dependencies between tasks\n"
    prompt+="4. Mark tasks that can run in parallel with (P)\n"
    prompt+="5. Include validation criteria for each phase\n"
    prompt+="6. Note any decisions that need human input with [HUMAN DECISION NEEDED]\n"

    echo -e "$prompt"
}

# Build a synthesis prompt from two plan outputs
generate_synthesis_prompt() {
    local claude_plan="$1"
    local codex_plan="$2"

    local prompt="You are synthesizing two independent execution plans into one.\n\n"
    prompt+="## Plan A (Claude)\n\n$(cat "$claude_plan")\n\n"
    prompt+="## Plan B (Codex)\n\n$(cat "$codex_plan")\n\n"
    prompt+="## Instructions\n\n"
    prompt+="Produce a merged execution plan that:\n"
    prompt+="1. **Agreements** — Where both plans agree, state the consensus approach\n"
    prompt+="2. **Disagreements** — Where they differ, present both options with [HUMAN DECISION NEEDED]\n"
    prompt+="3. **Unique insights** — Capture anything one plan found that the other missed\n"
    prompt+="4. Use EARS format for all acceptance criteria\n"
    prompt+="5. Preserve task dependency ordering\n"
    prompt+="6. Mark parallel-safe tasks with (P)\n\n"
    prompt+="Output the merged plan as a single markdown document.\n"

    echo -e "$prompt"
}

# --- Review Prompt Builders ---

# Build an adversarial plan review prompt from workstream context
generate_plan_review_prompt() {
    local ws_name="$1"
    local ws_dir
    ws_dir="$(get_workstream_dir "$ws_name")"
    local docs_dir
    docs_dir="$(get_docs_dir)"
    local exec_dir
    exec_dir="$(get_exec_dir "$ws_name")"

    local prompt="You are performing an adversarial review of the planning documents for workstream '$ws_name'.\n\n"
    prompt+="Your job is to find contradictions, gaps, ambiguities, and missing coverage BEFORE implementation begins.\n\n"

    # Load project-level context
    if [[ -f "$docs_dir/INVARIANTS.md" ]]; then
        prompt+="## Project Invariants\n\n$(cat "$docs_dir/INVARIANTS.md")\n\n"
    fi
    if [[ -f "$docs_dir/ARCHITECTURE.md" ]]; then
        prompt+="## Architecture\n\n$(cat "$docs_dir/ARCHITECTURE.md")\n\n"
    fi

    # Load workstream context
    if [[ -f "$ws_dir/SPEC.md" ]]; then
        prompt+="## Specification\n\n$(cat "$ws_dir/SPEC.md")\n\n"
    fi
    if [[ -f "$ws_dir/PLAN.md" ]]; then
        prompt+="## Plan\n\n$(cat "$ws_dir/PLAN.md")\n\n"
    fi
    if [[ -f "$ws_dir/SHARED-CONTEXT.md" ]]; then
        prompt+="## Shared Context\n\n$(cat "$ws_dir/SHARED-CONTEXT.md")\n\n"
    fi
    if [[ -f "$ws_dir/NARRATIVE.md" ]]; then
        prompt+="## Narrative\n\n$(cat "$ws_dir/NARRATIVE.md")\n\n"
    fi

    # Load execution plans if they exist
    local latest_synth
    latest_synth=$(ls -t "$exec_dir"/synthesized-*.md 2>/dev/null | head -1 || true)
    if [[ -f "$exec_dir/synthesized.md" ]]; then
        prompt+="## Synthesized Execution Plan\n\n$(cat "$exec_dir/synthesized.md")\n\n"
    elif [[ -n "$latest_synth" ]]; then
        prompt+="## Synthesized Execution Plan\n\n$(cat "$latest_synth")\n\n"
    else
        # Fall back to individual agent plans
        local latest_claude
        latest_claude=$(ls -t "$exec_dir"/claude-plan-*.md 2>/dev/null | head -1 || true)
        if [[ -n "$latest_claude" ]]; then
            prompt+="## Claude Execution Plan\n\n$(cat "$latest_claude")\n\n"
        fi
        local latest_codex
        latest_codex=$(ls -t "$exec_dir"/codex-plan-*.md 2>/dev/null | head -1 || true)
        if [[ -n "$latest_codex" ]]; then
            prompt+="## Codex Execution Plan\n\n$(cat "$latest_codex")\n\n"
        fi
    fi

    prompt+="## Review Instructions\n\n"
    prompt+="Perform a thorough adversarial review. For each finding, cite the specific file and section.\n\n"
    prompt+="### 1. Consistency Check\n"
    prompt+="- Contradictions between SPEC and PLAN\n"
    prompt+="- Requirements in PLAN not covered by SPEC\n"
    prompt+="- Orphaned EARS acceptance criteria with no test mapping\n"
    prompt+="- Scope gaps — what's unspecified that should be\n\n"
    prompt+="### 2. EARS Format Audit\n"
    prompt+="For every acceptance criterion in SPEC.md:\n"
    prompt+="- Is it in EARS format (WHEN/WHILE/IF/WHERE/SHALL)?\n"
    prompt+="- Does it have an AC-N.M identifier?\n"
    prompt+="- Does the Traces To column reference a test?\n"
    prompt+="- Is the criterion testable (not vague)?\n\n"
    prompt+="### 3. Completeness Check\n"
    prompt+="- All phases in PLAN.md have validation criteria\n"
    prompt+="- All FR/NFR in SPEC.md have EARS acceptance criteria\n"
    prompt+="- Error handling specified for all failure modes\n"
    prompt+="- Dependencies between workstreams documented\n\n"
    prompt+="### 4. Invariant Alignment\n"
    prompt+="- Does the plan honor every invariant in INVARIANTS.md?\n"
    prompt+="- Are there implicit assumptions that contradict invariants?\n\n"
    prompt+="### 5. Risk Assessment\n"
    prompt+="- Underestimated complexity\n"
    prompt+="- Missing error cases\n"
    prompt+="- Dependency risks\n"
    prompt+="- Ambiguous requirements that could be interpreted multiple ways\n\n"
    prompt+="## Output Format\n\n"
    prompt+="Structure your findings as:\n\n"
    prompt+="**Critical** (blocks implementation):\n"
    prompt+="- Finding with \`file:section\` reference\n\n"
    prompt+="**Major** (should fix before implementation):\n"
    prompt+="- Finding with \`file:section\` reference\n\n"
    prompt+="**Minor** (nice to fix):\n"
    prompt+="- Finding with \`file:section\` reference\n\n"
    prompt+="**Strengths** (what's done well):\n"
    prompt+="- Observation\n"

    echo -e "$prompt"
}

# Build a synthesis prompt from two plan review outputs
generate_review_synthesis_prompt() {
    local claude_review="$1"
    local codex_review="$2"

    local prompt="You are synthesizing two independent adversarial reviews of a workstream plan into unified, actionable feedback.\n\n"
    prompt+="## Review A (Claude)\n\n$(cat "$claude_review")\n\n"
    prompt+="## Review B (Codex)\n\n$(cat "$codex_review")\n\n"
    prompt+="## Instructions\n\n"
    prompt+="Produce a merged review that:\n"
    prompt+="1. **Agreed findings** — Issues both reviewers flagged. These are high-confidence.\n"
    prompt+="2. **Unique findings** — Issues only one reviewer caught. Assess validity.\n"
    prompt+="3. **Conflicting assessments** — Where reviewers disagree on severity or correctness.\n"
    prompt+="4. Deduplicate — merge findings about the same issue into one entry.\n"
    prompt+="5. Preserve severity levels: Critical > Major > Minor.\n"
    prompt+="6. For each finding, include the specific fix needed (not just the problem).\n\n"
    prompt+="## Output Format\n\n"
    prompt+="**Critical** (blocks implementation):\n"
    prompt+="- Finding + specific fix needed\n\n"
    prompt+="**Major** (should fix before implementation):\n"
    prompt+="- Finding + specific fix needed\n\n"
    prompt+="**Minor** (nice to fix):\n"
    prompt+="- Finding + specific fix needed\n\n"
    prompt+="**Strengths** (confirmed by both reviewers):\n"
    prompt+="- Observation\n"

    echo -e "$prompt"
}

# Build an adversarial code review prompt from workstream context + code
generate_code_review_prompt() {
    local ws_name="$1"
    local task_name="${2:-}"
    local ws_dir
    ws_dir="$(get_workstream_dir "$ws_name")"
    local docs_dir
    docs_dir="$(get_docs_dir)"
    local repo_root
    repo_root="$(get_repo_root)"

    local prompt="You are performing an adversarial code review for workstream '$ws_name'"
    [[ -n "$task_name" ]] && prompt+=" task '$task_name'"
    prompt+=".\n\n"
    prompt+="Review the implementation against the spec, not just for code quality.\n\n"

    # Load project-level context
    if [[ -f "$docs_dir/INVARIANTS.md" ]]; then
        prompt+="## Project Invariants\n\n$(cat "$docs_dir/INVARIANTS.md")\n\n"
    fi

    # Load workstream context
    if [[ -f "$ws_dir/SPEC.md" ]]; then
        prompt+="## Specification\n\n$(cat "$ws_dir/SPEC.md")\n\n"
    fi
    if [[ -f "$ws_dir/PLAN.md" ]]; then
        prompt+="## Plan\n\n$(cat "$ws_dir/PLAN.md")\n\n"
    fi
    if [[ -f "$ws_dir/SHARED-CONTEXT.md" ]]; then
        prompt+="## Shared Context\n\n$(cat "$ws_dir/SHARED-CONTEXT.md")\n\n"
    fi

    # Load task-specific context if available
    if [[ -n "$task_name" ]]; then
        local task_dir
        task_dir="$(get_task_dir "$ws_name" "$task_name")"
        [[ -f "$task_dir/CONTEXT.md" ]] && prompt+="## Task Context\n\n$(cat "$task_dir/CONTEXT.md")\n\n"
        [[ -f "$task_dir/VALIDATION.md" ]] && prompt+="## Validation Criteria\n\n$(cat "$task_dir/VALIDATION.md")\n\n"
    fi

    # Include file ownership map so the agent knows what to review
    prompt+="## Review Instructions\n\n"
    prompt+="### 1. Fresh Eyes Pass\n"
    prompt+="Read the code without spec context first. What jumps out?\n\n"
    prompt+="### 2. Spec Alignment\n"
    prompt+="Walk through each EARS acceptance criterion in SPEC.md. Is it implemented correctly?\n"
    prompt+="Flag any AC that is not covered or incorrectly implemented.\n\n"
    prompt+="### 3. Invariant Compliance\n"
    prompt+="Does the code honor every invariant in INVARIANTS.md?\n\n"
    prompt+="### 4. Design Review\n"
    prompt+="- Right abstractions? Right boundaries?\n"
    prompt+="- Essential complexity only? No unnecessary abstractions?\n"
    prompt+="- API ergonomics — is the public interface hard to misuse?\n\n"
    prompt+="### 5. Error Handling\n"
    prompt+="- Are all error paths covered?\n"
    prompt+="- Do errors include what/where/why/hint per SPEC?\n"
    prompt+="- Can an LLM caller recover from every error?\n\n"
    prompt+="### 6. Testing Review\n"
    prompt+="- Do tests cover EARS criteria?\n"
    prompt+="- Edge cases handled?\n"
    prompt+="- Are tests testing behavior, not implementation?\n\n"
    prompt+="### 7. LLM-Specific Traps\n"
    prompt+="- Hallucinated APIs or crate functions that don't exist?\n"
    prompt+="- Silent failures that would confuse an LLM caller?\n"
    prompt+="- Unnecessary complexity?\n\n"
    prompt+="## Output Format\n\n"
    prompt+="**Verdict:** Approve / Request Changes / Needs Discussion\n\n"
    prompt+="**Critical** (must fix before merge):\n"
    prompt+="- Finding with \`file:line\` reference\n\n"
    prompt+="**Major** (should fix):\n"
    prompt+="- Finding with \`file:line\` reference\n\n"
    prompt+="**Minor** (nice to fix):\n"
    prompt+="- Finding with \`file:line\` reference\n\n"
    prompt+="**Strengths** (what's done well):\n"
    prompt+="- Observation\n"

    echo -e "$prompt"
}

# Build a synthesis prompt from two code review outputs
generate_code_review_synthesis_prompt() {
    local claude_review="$1"
    local codex_review="$2"

    local prompt="You are synthesizing two independent adversarial code reviews into unified, actionable feedback.\n\n"
    prompt+="## Review A (Claude)\n\n$(cat "$claude_review")\n\n"
    prompt+="## Review B (Codex)\n\n$(cat "$codex_review")\n\n"
    prompt+="## Instructions\n\n"
    prompt+="Produce a merged code review that:\n"
    prompt+="1. **Agreed findings** — Issues both reviewers flagged. These are high-confidence.\n"
    prompt+="2. **Unique findings** — Issues only one reviewer caught. Assess validity.\n"
    prompt+="3. **Conflicting assessments** — Where reviewers disagree on severity or correctness.\n"
    prompt+="4. Deduplicate — merge findings about the same issue into one entry.\n"
    prompt+="5. Preserve severity levels: Critical > Major > Minor.\n"
    prompt+="6. For each finding, include the specific fix needed with \`file:line\` references.\n"
    prompt+="7. Drop any finding that is incorrect or based on misunderstanding the code.\n\n"
    prompt+="## Output Format\n\n"
    prompt+="**Verdict:** Approve / Request Changes / Needs Discussion\n\n"
    prompt+="**Critical** (must fix):\n"
    prompt+="- [ ] Finding + \`file:line\` + specific fix needed\n\n"
    prompt+="**Major** (should fix):\n"
    prompt+="- [ ] Finding + \`file:line\` + specific fix needed\n\n"
    prompt+="**Minor** (nice to fix):\n"
    prompt+="- [ ] Finding + \`file:line\` + specific fix needed\n\n"
    prompt+="**Strengths** (confirmed by both reviewers):\n"
    prompt+="- Observation\n"

    echo -e "$prompt"
}

# Build a prompt to fix code based on review findings
generate_review_fix_prompt() {
    local ws_name="$1"
    local task_name="${2:-}"
    local findings_file="$3"
    local ws_dir
    ws_dir="$(get_workstream_dir "$ws_name")"
    local docs_dir
    docs_dir="$(get_docs_dir)"

    local prompt="You are fixing code based on review findings for workstream '$ws_name'"
    [[ -n "$task_name" ]] && prompt+=" task '$task_name'"
    prompt+=".\n\n"

    # Load invariants so the agent doesn't violate them while fixing
    if [[ -f "$docs_dir/INVARIANTS.md" ]]; then
        prompt+="## Project Invariants (do not violate)\n\n$(cat "$docs_dir/INVARIANTS.md")\n\n"
    fi

    # Load spec for reference
    if [[ -f "$ws_dir/SPEC.md" ]]; then
        prompt+="## Specification\n\n$(cat "$ws_dir/SPEC.md")\n\n"
    fi

    # Task context if scoped
    if [[ -n "$task_name" ]]; then
        local task_dir
        task_dir="$(get_task_dir "$ws_name" "$task_name")"
        [[ -f "$task_dir/CONTEXT.md" ]] && prompt+="## Task Context\n\n$(cat "$task_dir/CONTEXT.md")\n\n"
    fi

    prompt+="## Review Findings to Fix\n\n$(cat "$findings_file")\n\n"

    prompt+="## Instructions\n\n"
    prompt+="1. Fix all **Critical** findings. These are blocking.\n"
    prompt+="2. Fix all **Major** findings.\n"
    prompt+="3. Fix **Minor** findings if they are straightforward.\n"
    prompt+="4. For each finding, make the minimal change needed. Do not refactor surrounding code.\n"
    prompt+="5. Run \`cargo test\` after changes to ensure nothing is broken.\n"
    prompt+="6. If a finding is wrong or already fixed, skip it.\n\n"
    prompt+="When done, output a summary of what you fixed and what you skipped (with reasons).\n"

    echo -e "$prompt"
}

# Build a consolidation prompt from workstream context + recent changes
generate_consolidation_prompt() {
    local ws_name="$1"
    local ws_dir
    ws_dir="$(get_workstream_dir "$ws_name")"
    local docs_dir
    docs_dir="$(get_docs_dir)"
    local repo_root
    repo_root="$(get_repo_root)"

    local prompt="You are consolidating learnings from recent work on workstream '$ws_name'.\n\n"
    prompt+="Your job is to update durable documentation so future conversations have full context.\n\n"

    # Load workstream context
    if [[ -f "$ws_dir/PLAN.md" ]]; then
        prompt+="## Current Plan\n\n$(cat "$ws_dir/PLAN.md")\n\n"
    fi
    if [[ -f "$ws_dir/SPEC.md" ]]; then
        prompt+="## Specification\n\n$(cat "$ws_dir/SPEC.md")\n\n"
    fi
    if [[ -f "$ws_dir/SHARED-CONTEXT.md" ]]; then
        prompt+="## Shared Context\n\n$(cat "$ws_dir/SHARED-CONTEXT.md")\n\n"
    fi

    # Load project-level context for promotion decisions
    if [[ -f "$docs_dir/INVARIANTS.md" ]]; then
        prompt+="## Project Invariants\n\n$(cat "$docs_dir/INVARIANTS.md")\n\n"
    fi
    if [[ -f "$docs_dir/ARCHITECTURE.md" ]]; then
        prompt+="## Architecture\n\n$(cat "$docs_dir/ARCHITECTURE.md")\n\n"
    fi

    # Include recent git history
    local git_log
    git_log=$(cd "$repo_root" && git log --oneline -20 2>/dev/null || echo "(no git history)")
    prompt+="## Recent Git History\n\n\`\`\`\n$git_log\n\`\`\`\n\n"

    local git_diff_stat
    git_diff_stat=$(cd "$repo_root" && git diff --stat HEAD~10 HEAD 2>/dev/null || echo "(no diff)")
    prompt+="## Recent Changes (stat)\n\n\`\`\`\n$git_diff_stat\n\`\`\`\n\n"

    prompt+="## Consolidation Instructions\n\n"
    prompt+="### 1. Update PLAN.md\n"
    prompt+="- Mark completed milestones with \`[x]\`\n"
    prompt+="- Update phase status (Planned → In Progress → Complete)\n"
    prompt+="- Add any new milestones discovered during implementation\n\n"
    prompt+="### 2. Update SHARED-CONTEXT.md\n"
    prompt+="- Add decisions made during implementation (with rationale)\n"
    prompt+="- Add patterns established\n"
    prompt+="- Add known issues / tech debt discovered\n"
    prompt+="- Update file ownership if new files were created\n\n"
    prompt+="### 3. Update SPEC.md (if needed)\n"
    prompt+="- Update if requirements changed during implementation\n"
    prompt+="- Add any new acceptance criteria discovered\n\n"
    prompt+="### 4. Check for Promotions\n"
    prompt+="Should any learnings be promoted to project-level docs?\n"
    prompt+="- INVARIANTS.md — new constraint that applies to all future work\n"
    prompt+="- ARCHITECTURE.md — new interface or system boundary\n"
    prompt+="- CLAUDE.md / AGENTS.md — new convention or build command\n\n"
    prompt+="### 5. Clean Up\n"
    prompt+="- Identify ephemeral task docs whose knowledge is now in SHARED-CONTEXT.md\n"
    prompt+="- Identify exec/ iteration artifacts that can be archived (keep summaries)\n\n"
    prompt+="## Output\n\n"
    prompt+="For each file you would update, show the specific changes.\n"
    prompt+="Mark promotions with [PROMOTE TO <target>].\n"
    prompt+="Mark cleanup candidates with [ARCHIVE].\n"

    echo -e "$prompt"
}

# --- Execution Helpers (Track C) ---

# Build an execution prompt from task context + previous errors
generate_execution_prompt() {
    local ws_name="$1"
    local task_name="${2:-}"
    local prev_error="${3:-}"
    local ws_dir
    ws_dir="$(get_workstream_dir "$ws_name")"
    local exec_dir
    exec_dir="$(get_exec_dir "$ws_name")"

    local prompt="You are implementing workstream '$ws_name'"
    [[ -n "$task_name" ]] && prompt+=" task '$task_name'"
    prompt+=".\n\n"

    # Prefer synthesized plan, fall back to PLAN.md + SPEC.md
    if [[ -f "$exec_dir/synthesized.md" ]]; then
        prompt+="## Execution Plan\n\n$(cat "$exec_dir/synthesized.md")\n\n"
    else
        local latest_synth
        latest_synth=$(ls -t "$exec_dir"/synthesized-*.md 2>/dev/null | head -1 || true)
        if [[ -n "$latest_synth" ]]; then
            prompt+="## Execution Plan\n\n$(cat "$latest_synth")\n\n"
        else
            [[ -f "$ws_dir/PLAN.md" ]] && prompt+="## Plan\n\n$(cat "$ws_dir/PLAN.md")\n\n"
            [[ -f "$ws_dir/SPEC.md" ]] && prompt+="## Spec\n\n$(cat "$ws_dir/SPEC.md")\n\n"
        fi
    fi

    # Load task-specific context if available
    if [[ -n "$task_name" ]]; then
        local task_dir
        task_dir="$(get_task_dir "$ws_name" "$task_name")"
        [[ -f "$task_dir/CONTEXT.md" ]] && prompt+="## Task Context\n\n$(cat "$task_dir/CONTEXT.md")\n\n"
        [[ -f "$task_dir/VALIDATION.md" ]] && prompt+="## Validation Criteria (Definition of Done)\n\n$(cat "$task_dir/VALIDATION.md")\n\n"
    fi

    # Add previous error context if this is a retry
    if [[ -n "$prev_error" ]]; then
        prompt+="## Previous Attempt Failed\n\n"
        prompt+="The previous attempt failed with these errors. Fix specifically these issues:\n\n"
        prompt+="$prev_error\n\n"
    fi

    prompt+="## Instructions\n\n"
    prompt+="1. Implement the changes described above\n"
    prompt+="2. Write tests for each acceptance criterion\n"
    prompt+="3. Run the test suite to verify\n"
    prompt+="4. If all tests pass and validation criteria are met, output COMPLETE on its own line\n"

    echo -e "$prompt"
}

# Parse validate.sh output for failures
capture_validation_failures() {
    local validate_output="$1"
    echo "$validate_output" | grep -E "(FAIL|MISSING|ERROR|PENDING)" || echo "Validation did not pass (see full output above)"
}

# Save iteration output to exec directory
save_iteration() {
    local ws_name="$1"
    local iteration="$2"
    local output="$3"
    local exec_dir
    exec_dir="$(get_exec_dir "$ws_name")"
    local timestamp
    timestamp=$(date +%Y%m%d-%H%M%S)

    echo "$output" > "$exec_dir/iteration-${iteration}-${timestamp}.md"
    log_info "Saved iteration $iteration to $exec_dir/iteration-${iteration}-${timestamp}.md"
}
