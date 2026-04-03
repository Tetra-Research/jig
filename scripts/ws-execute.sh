#!/bin/bash
# ws-execute.sh — Iterative execution with fresh-context retries (Ralph-loop pattern)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-execute <workstream> [task] [--agent claude|codex] [--max-iter 5]"
    echo ""
    echo "Runs an agent in a fresh-context iteration loop."
    echo "Each iteration: execute -> validate -> if fail, feed errors to next iteration."
    echo ""
    echo "Options:"
    echo "  --agent        Agent to use (default: claude)"
    echo "  --max-iter     Maximum iterations (default: 5)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
task_name=""
agent="claude"
max_iter=5

while [[ $# -gt 0 ]]; do
    case "$1" in
        --agent) agent="$2"; shift 2 ;;
        --max-iter) max_iter="$2"; shift 2 ;;
        --help|-h) usage ;;
        *)
            if [[ -z "$task_name" ]]; then
                task_name="$1"; shift
            else
                usage
            fi
            ;;
    esac
done

ws_dir="$(get_workstream_dir "$ws_name")"
exec_dir="$(get_exec_dir "$ws_name")"
repo_root="$(get_repo_root)"
work_dir="$repo_root"

if [[ ! -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' not found at $ws_dir"
    exit 1
fi

# If task specified, check for its context
if [[ -n "$task_name" ]]; then
    task_dir="$(get_task_dir "$ws_name" "$task_name")"
    if [[ ! -d "$task_dir" ]]; then
        log_warn "Task dir $task_dir not found, using workstream context only"
    fi
fi

log_info "Starting execution loop for '$ws_name'${task_name:+ task '$task_name'}"
log_info "Agent: $agent | Max iterations: $max_iter"
echo ""

prev_error=""
completed=false
timestamp_start=$(date +%Y%m%d-%H%M%S)

for i in $(seq 1 "$max_iter"); do
    log_info "=== Iteration $i/$max_iter ==="

    # Build prompt with previous failure context
    prompt=$(generate_execution_prompt "$ws_name" "$task_name" "$prev_error")

    # Run agent (fresh context each time)
    local_timestamp=$(date +%Y%m%d-%H%M%S)
    output_file="$exec_dir/iteration-${i}-${local_timestamp}.md"
    output=""

    case "$agent" in
        claude)
            log_info "Running Claude..."
            output=$(claude -p "$prompt" --cwd "$work_dir" 2>&1) || true
            ;;
        codex)
            log_info "Running Codex..."
            output=$(codex exec --full-auto "$prompt" 2>&1) || true
            ;;
        *)
            log_error "Unknown agent: $agent (use claude or codex)"
            exit 1
            ;;
    esac

    # Save iteration output
    echo "$output" > "$output_file"
    log_info "Output saved to $output_file"

    # Check for COMPLETE token in output
    if echo "$output" | grep -q "^COMPLETE$"; then
        log_info "Agent declared COMPLETE"
    fi

    # Run validation
    log_info "Running validation..."
    validate_output=""
    if [[ -f "$SCRIPT_DIR/validate.sh" ]]; then
        validate_output=$("$SCRIPT_DIR/validate.sh" "$ws_name" ${task_name:+"$task_name"} 2>&1) || true
        echo "$validate_output"

        if echo "$validate_output" | grep -q "READY"; then
            log_info "Validation PASSED on iteration $i"
            completed=true
            break
        else
            log_warn "Validation did not pass"
            prev_error=$(capture_validation_failures "$validate_output")
            log_info "Feeding failures to next iteration..."
        fi
    else
        log_warn "No validate.sh found, skipping validation"
        # Without validation, check for COMPLETE token as fallback
        if echo "$output" | grep -q "^COMPLETE$"; then
            completed=true
            break
        fi
    fi

    echo ""
done

# Write execution summary
summary_file="$exec_dir/execution-summary-${timestamp_start}.md"
cat > "$summary_file" << EOF
# Execution Summary

- **Workstream:** $ws_name
- **Task:** ${task_name:-"(none)"}
- **Agent:** $agent
- **Iterations:** $i/$max_iter
- **Status:** $(if $completed; then echo "COMPLETE"; else echo "INCOMPLETE"; fi)
- **Started:** $timestamp_start

## Iteration Log

$(for j in $(seq 1 "$i"); do
    iter_file=$(ls -t "$exec_dir"/iteration-${j}-*.md 2>/dev/null | head -1)
    if [[ -n "$iter_file" ]]; then
        echo "### Iteration $j"
        echo "File: $(basename "$iter_file")"
        echo ""
    fi
done)
EOF

echo ""
if $completed; then
    log_info "COMPLETE after $i iteration(s)"
    log_info "Next: /ws-review $ws_name${task_name:+ $task_name}"
else
    log_warn "INCOMPLETE after $max_iter iterations"
    log_info "Review iteration outputs in $exec_dir/"
    log_info "You can re-run with: just execute $ws_name${task_name:+ $task_name} --max-iter $max_iter"
fi
log_info "Summary: $summary_file"
