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
