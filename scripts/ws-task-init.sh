#!/bin/bash
# ws-task-init.sh — Parse PLAN.md phases into per-phase task directories
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-task-init <workstream> [--force]"
    echo ""
    echo "Parses PLAN.md phases into task directories under tasks/phase-N/"
    echo "Each task gets CONTEXT.md (phase plan + relevant SPEC ACs) and VALIDATION.md"
    echo ""
    echo "Options:"
    echo "  --force    Overwrite existing task directories"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
force=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --force) force=true; shift ;;
        *) usage ;;
    esac
done

ws_dir="$(get_workstream_dir "$ws_name")"
docs_dir="$(get_docs_dir)"
plan_file="$ws_dir/PLAN.md"
spec_file="$ws_dir/SPEC.md"

if [[ ! -f "$plan_file" ]]; then
    log_error "PLAN.md not found at $plan_file"
    exit 1
fi

# --- Parse phases from PLAN.md ---

# Find all phase headers: "### Phase N: Title"
phase_lines=()
phase_numbers=()
phase_titles=()
while IFS= read -r line; do
    if [[ "$line" =~ ^###[[:space:]]+Phase[[:space:]]+([0-9]+):[[:space:]]+(.*) ]]; then
        phase_numbers+=("${BASH_REMATCH[1]}")
        phase_titles+=("${BASH_REMATCH[2]}")
        phase_lines+=("$(grep -n "^### Phase ${BASH_REMATCH[1]}:" "$plan_file" | head -1 | cut -d: -f1)")
    fi
done < "$plan_file"

total_phases=${#phase_numbers[@]}
if [[ $total_phases -eq 0 ]]; then
    log_error "No phases found in $plan_file"
    exit 1
fi

log_info "Found $total_phases phases in PLAN.md"

# --- Extract each phase's content ---

total_lines=$(wc -l < "$plan_file")

for i in $(seq 0 $((total_phases - 1))); do
    num="${phase_numbers[$i]}"
    title="${phase_titles[$i]}"
    start="${phase_lines[$i]}"

    # End is next phase start - 1, or end of Phases section
    if [[ $i -lt $((total_phases - 1)) ]]; then
        end=$((${phase_lines[$((i + 1))]} - 1))
    else
        # Find the next ## header after this phase, or EOF
        end=$(tail -n +"$((start + 1))" "$plan_file" | grep -n "^## " | head -1 | cut -d: -f1 || true)
        if [[ -n "$end" ]]; then
            end=$((start + end - 1))
        else
            end=$total_lines
        fi
    fi

    # Extract phase content
    phase_content=$(sed -n "${start},${end}p" "$plan_file")

    # Extract "Traces to:" line for FR/NFR references
    traces_line=$(echo "$phase_content" | grep "^Traces to:" | head -1 || true)
    fr_refs=()
    if [[ -n "$traces_line" ]]; then
        # Parse FR-N and NFR-N references
        while IFS= read -r ref; do
            fr_refs+=("$ref")
        done < <(echo "$traces_line" | grep -oE '(FR|NFR)-[0-9]+' | sort -u)
    fi

    # --- Create task directory ---
    task_dir="$ws_dir/tasks/phase-${num}"

    if [[ -d "$task_dir" && "$force" != true ]]; then
        log_warn "Task dir $task_dir already exists (use --force to overwrite)"
        continue
    fi

    mkdir -p "$task_dir"

    # --- Build CONTEXT.md ---
    {
        echo "# Phase ${num}: ${title}"
        echo ""
        echo "> Workstream: ${ws_name}"
        echo "> Generated: $(date +%Y-%m-%d)"
        echo "> Source: PLAN.md"
        echo ""
        echo "## Phase Plan"
        echo ""
        echo "$phase_content" | sed 's/^### /## /'
        echo ""

        # Include relevant FR/NFR sections from SPEC.md
        if [[ -f "$spec_file" && ${#fr_refs[@]} -gt 0 ]]; then
            echo "## Relevant Acceptance Criteria"
            echo ""
            echo "Extracted from SPEC.md for: ${fr_refs[*]}"
            echo ""

            for ref in "${fr_refs[@]}"; do
                # Determine the AC prefix for this FR/NFR
                if [[ "$ref" =~ ^NFR-([0-9]+)$ ]]; then
                    ac_prefix="AC-N${BASH_REMATCH[1]}"
                elif [[ "$ref" =~ ^FR-([0-9]+)$ ]]; then
                    ac_prefix="AC-${BASH_REMATCH[1]}"
                else
                    continue
                fi

                # Find the FR/NFR section header
                section_line=$(grep -n "^#### ${ref}:" "$spec_file" | head -1 | cut -d: -f1 || true)
                if [[ -z "$section_line" ]]; then
                    continue
                fi

                # Extract the section header + description
                section_header=$(sed -n "${section_line}p" "$spec_file")
                echo "### ${section_header##### }"
                echo ""

                # Extract all AC rows for this section
                ac_rows=$(grep -E "^\| *${ac_prefix}\." "$spec_file" || true)
                if [[ -n "$ac_rows" ]]; then
                    echo "| ID | Type | Criterion | Traces To |"
                    echo "|----|------|-----------|-----------|"
                    echo "$ac_rows"
                    echo ""
                fi
            done
        fi

        # Include dependency context
        echo "## Execution Context"
        echo ""
        if [[ "$num" -gt 1 ]]; then
            echo "This phase builds on Phase $((num - 1)). Assume all prior phase artifacts exist and tests pass."
        else
            echo "This is the first phase. No prior artifacts exist — bootstrap from scratch."
        fi
        echo ""

        # Include key invariants reference
        if [[ -f "$docs_dir/INVARIANTS.md" ]]; then
            echo "## Invariants"
            echo ""
            echo "Refer to \`docs/INVARIANTS.md\` for project-wide constraints that must be honored."
            echo ""
        fi

        # Include architecture reference
        if [[ -f "$docs_dir/ARCHITECTURE.md" ]]; then
            echo "## Architecture"
            echo ""
            echo "Refer to \`docs/ARCHITECTURE.md\` for module boundaries and design decisions."
            echo ""
        fi
    } > "$task_dir/CONTEXT.md"

    # --- Build VALIDATION.md ---
    {
        echo "# VALIDATION.md"
        echo ""
        echo "> Workstream: ${ws_name}"
        echo "> Task: phase-${num}"
        echo "> Last verified: $(date +%Y-%m-%d)"
        echo ""

        # Extract validation criteria from phase content
        echo "## Phase Validation Criteria"
        echo ""
        echo "From PLAN.md Phase ${num}:"
        echo ""
        in_validation=false
        while IFS= read -r line; do
            if [[ "$line" =~ ^####[[:space:]]+Validation[[:space:]]+Criteria ]]; then
                in_validation=true
                continue
            fi
            if [[ "$in_validation" == true ]]; then
                if [[ "$line" =~ ^####[[:space:]] || "$line" =~ ^###[[:space:]] || "$line" == "---" ]]; then
                    break
                fi
                [[ -n "$line" ]] && echo "$line"
            fi
        done <<< "$phase_content"
        echo ""

        # Build AC traceability table
        echo "## Spec Requirements -> Tests"
        echo ""
        echo "| Criterion | EARS Type | Source | Test | Status |"
        echo "|-----------|-----------|--------|------|--------|"

        ac_count=0
        if [[ -f "$spec_file" && ${#fr_refs[@]} -gt 0 ]]; then
            for ref in "${fr_refs[@]}"; do
                if [[ "$ref" =~ ^NFR-([0-9]+)$ ]]; then
                    ac_prefix="AC-N${BASH_REMATCH[1]}"
                elif [[ "$ref" =~ ^FR-([0-9]+)$ ]]; then
                    ac_prefix="AC-${BASH_REMATCH[1]}"
                else
                    continue
                fi

                while IFS='|' read -r _ ac_id ac_type ac_criterion ac_traces _; do
                    ac_id=$(echo "$ac_id" | xargs)
                    ac_type=$(echo "$ac_type" | xargs)
                    [[ -z "$ac_id" ]] && continue
                    ref_lower=$(echo "$ref" | tr '[:upper:]' '[:lower:]')
                    ac_lower=$(echo "$ac_id" | tr '[:upper:]' '[:lower:]' | sed 's/ac-/ac_/g; s/\./_/g; s/-/_/g')
                    echo "| ${ac_id} | ${ac_type} | SPEC.md ${ref} | \`spec::${ref_lower}::${ac_lower}\` | PENDING |"
                    ((ac_count++))
                done < <(grep -E "^\| *${ac_prefix}\." "$spec_file" || true)
            done
        fi

        if [[ $ac_count -eq 0 ]]; then
            echo "| <!-- AC-N.N --> | <!-- type --> | SPEC.md | \`spec::...\` | PENDING |"
        fi

        echo ""
        echo "## Coverage Summary"
        echo ""
        echo "- Spec criteria: 0/${ac_count} covered"
        echo "- Phase validation criteria: 0/$(echo "$phase_content" | grep -c "^- " || echo "0") covered"
        echo ""
        echo "## Gaps"
        echo ""
        echo "All criteria need test implementations."
    } > "$task_dir/VALIDATION.md"

    log_info "Phase ${num}: ${title} → tasks/phase-${num}/ (${#fr_refs[@]} FR/NFR refs, ${ac_count} ACs)"
done

echo ""
log_info "Task directories created under $ws_dir/tasks/"
log_info ""
log_info "Next steps:"
log_info "  1. Review CONTEXT.md and VALIDATION.md in each phase dir"
log_info "  2. Run: ./scripts/ws-execute.sh $ws_name phase-1 --max-iter 5"
log_info "  3. After phase passes: ./scripts/ws-execute.sh $ws_name phase-2 --max-iter 5"
log_info "  4. Continue through all phases sequentially"
