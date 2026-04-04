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

# --- Progress helpers ---

elapsed_since() {
    local start_ts="$1"
    local now_ts
    now_ts=$(date +%s)
    local diff=$((now_ts - start_ts))
    local mins=$((diff / 60))
    local secs=$((diff % 60))
    if [[ $mins -gt 0 ]]; then
        echo "${mins}m ${secs}s"
    else
        echo "${secs}s"
    fi
}

# Snapshot git state for delta reporting
snapshot_git_state() {
    cd "$repo_root"
    echo "$(git diff --stat HEAD 2>/dev/null | tail -1 || true)"
}

count_src_files() {
    find "$repo_root/src" -name "*.rs" 2>/dev/null | wc -l | tr -d ' '
}

count_test_files() {
    find "$repo_root/tests" -name "*.rs" 2>/dev/null | wc -l | tr -d ' '
}

# --- Main execution loop ---

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ws-execute: $ws_name${task_name:+ / $task_name}"
echo "║  Agent: $agent | Max iterations: $max_iter"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

prev_error=""
completed=false
timestamp_start=$(date +%Y%m%d-%H%M%S)
loop_start_ts=$(date +%s)

for i in $(seq 1 "$max_iter"); do
    iter_start_ts=$(date +%s)

    echo "┌──────────────────────────────────────────────────────────"
    echo "│ Iteration $i/$max_iter  [$(date +%H:%M:%S)]"
    echo "└──────────────────────────────────────────────────────────"

    # Pre-iteration snapshot
    pre_src_count=$(count_src_files)
    pre_test_count=$(count_test_files)
    pre_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || echo "no tests yet")

    log_info "Pre-state: ${pre_src_count} src files, ${pre_test_count} test files"
    [[ -n "$pre_test_result" ]] && log_info "Pre-tests: $pre_test_result"

    if [[ -n "$prev_error" ]]; then
        log_info "Feeding previous failures to agent:"
        echo "$prev_error" | head -5 | sed 's/^/  │ /'
        echo ""
    fi

    # Build prompt with previous failure context
    prompt=$(generate_execution_prompt "$ws_name" "$task_name" "$prev_error")

    # Run agent (fresh context each time)
    local_timestamp=$(date +%Y%m%d-%H%M%S)
    output_file="$exec_dir/iteration-${i}-${local_timestamp}.md"
    output=""

    log_info "Starting $agent agent..."

    case "$agent" in
        claude)
            output=$(cd "$repo_root" && claude -p "$prompt" --permission-mode bypassPermissions --output-format json 2>&1) || true
            ;;
        codex)
            output=$(codex exec --full-auto "$prompt" 2>&1) || true
            ;;
        *)
            log_error "Unknown agent: $agent (use claude or codex)"
            exit 1
            ;;
    esac

    agent_elapsed=$(elapsed_since "$iter_start_ts")
    log_info "Agent finished in $agent_elapsed"

    # Save iteration output
    echo "$output" > "$output_file"
    log_info "Output: $output_file"

    # Post-iteration delta
    post_src_count=$(count_src_files)
    post_test_count=$(count_test_files)
    src_delta=$((post_src_count - pre_src_count))
    test_delta=$((post_test_count - pre_test_count))

    # Show what changed
    git_changes=$(cd "$repo_root" && git diff --stat HEAD 2>/dev/null | tail -1 || true)
    new_files=$(cd "$repo_root" && git ls-files --others --exclude-standard src/ tests/ 2>/dev/null | wc -l | tr -d ' ')

    echo ""
    log_info "Delta: src files ${pre_src_count}→${post_src_count} (+${src_delta}), test files ${pre_test_count}→${post_test_count} (+${test_delta}), new untracked: ${new_files}"
    [[ -n "$git_changes" ]] && log_info "Git: $git_changes"

    # Check for COMPLETE token in output
    if echo "$output" | grep -q "^COMPLETE$"; then
        log_info "Agent declared COMPLETE"
    fi

    # Run validation
    echo ""
    log_info "Running validation..."
    validate_output=""
    if [[ -f "$SCRIPT_DIR/validate.sh" ]]; then
        validate_output=$("$SCRIPT_DIR/validate.sh" "$ws_name" ${task_name:+"$task_name"} 2>&1) || true

        # Extract and display key validation metrics
        test_line=$(echo "$validate_output" | grep "^Tests:" || true)
        pass_line=$(echo "$validate_output" | grep "PASS:" | head -1 || true)
        pending_line=$(echo "$validate_output" | grep "PENDING:" || true)
        fail_line=$(echo "$validate_output" | grep "FAIL:" | head -1 || true)
        recommendation=$(echo "$validate_output" | grep "^Recommendation:" || true)

        # Show test results
        post_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || true)
        [[ -n "$post_test_result" ]] && log_info "Tests: $post_test_result"

        # Show VALIDATION.md status
        [[ -n "$pass_line" ]] && log_info "VALIDATION.md — $pass_line"
        [[ -n "$pending_line" ]] && log_info "VALIDATION.md — $pending_line"
        [[ -n "$fail_line" ]] && log_info "VALIDATION.md — $fail_line"

        iter_elapsed=$(elapsed_since "$iter_start_ts")
        total_elapsed=$(elapsed_since "$loop_start_ts")

        echo ""
        if echo "$validate_output" | grep -q "^Recommendation: READY$"; then
            echo "  ✓ Iteration $i PASSED  [$iter_elapsed iter / $total_elapsed total]"
            log_info "Validation PASSED on iteration $i"
            completed=true
            break
        else
            echo "  ✗ Iteration $i FAILED  [$iter_elapsed iter / $total_elapsed total]"
            log_warn "Validation did not pass"
            prev_error=$(capture_validation_failures "$validate_output")
            if [[ $i -lt $max_iter ]]; then
                log_info "Feeding failures to iteration $((i + 1))..."
            fi
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

# --- Execution summary ---

total_elapsed=$(elapsed_since "$loop_start_ts")
final_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || echo "no tests")

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
if $completed; then
    echo "║  COMPLETE — $ws_name${task_name:+ / $task_name}"
else
    echo "║  INCOMPLETE — $ws_name${task_name:+ / $task_name}"
fi
echo "║  Iterations: $i/$max_iter | Time: $total_elapsed"
echo "║  $final_test_result"
echo "╚══════════════════════════════════════════════════════════╝"

# Write execution summary
summary_file="$exec_dir/execution-summary-${timestamp_start}.md"
cat > "$summary_file" << EOF
# Execution Summary

- **Workstream:** $ws_name
- **Task:** ${task_name:-"(none)"}
- **Agent:** $agent
- **Iterations:** $i/$max_iter
- **Status:** $(if $completed; then echo "COMPLETE"; else echo "INCOMPLETE"; fi)
- **Total time:** $total_elapsed
- **Started:** $timestamp_start
- **Final tests:** $final_test_result

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
    log_info "Next: /ws-review $ws_name${task_name:+ $task_name}"
else
    log_warn "Review iteration outputs in $exec_dir/"
    log_info "Re-run: ./scripts/ws-execute.sh $ws_name${task_name:+ $task_name} --max-iter $max_iter"
fi
log_info "Summary: $summary_file"
