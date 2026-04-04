#!/bin/bash
# ws-execute.sh — Iterative execution with fresh-context retries (Ralph-loop pattern)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-execute <workstream> [task...] [--agent claude|codex] [--max-iter 5]"
    echo ""
    echo "Runs an agent in a fresh-context iteration loop."
    echo "Each iteration: execute -> validate -> if fail, feed errors to next iteration."
    echo ""
    echo "Multiple tasks run sequentially; stops on first failure."
    echo ""
    echo "Examples:"
    echo "  ws-execute core-engine phase-3                        # single phase"
    echo "  ws-execute core-engine phase-3 phase-4 phase-5        # sequential phases"
    echo "  ws-execute core-engine phase-3 phase-4 --max-iter 3   # 3 iterations per phase"
    echo ""
    echo "Options:"
    echo "  --agent        Agent to use (default: claude)"
    echo "  --max-iter     Maximum iterations per phase (default: 5)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
task_names=()
agent="claude"
max_iter=5

while [[ $# -gt 0 ]]; do
    case "$1" in
        --agent) agent="$2"; shift 2 ;;
        --max-iter) max_iter="$2"; shift 2 ;;
        --help|-h) usage ;;
        *) task_names+=("$1"); shift ;;
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

# Default to single empty task if none specified
if [[ ${#task_names[@]} -eq 0 ]]; then
    task_names=("")
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

# --- Run a single task through the iteration loop ---
# Sets: phase_completed, phase_iterations, phase_elapsed
run_task() {
    local task_name="$1"
    local task_label="${task_name:-(workstream)}"

    # Check task dir exists
    if [[ -n "$task_name" ]]; then
        local task_dir
        task_dir="$(get_task_dir "$ws_name" "$task_name")"
        if [[ ! -d "$task_dir" ]]; then
            log_warn "Task dir $task_dir not found, using workstream context only"
        fi
    fi

    echo ""
    echo "╔══════════════════════════════════════════════════════════╗"
    echo "║  Phase: $ws_name / $task_label"
    echo "║  Agent: $agent | Max iterations: $max_iter"
    echo "╚══════════════════════════════════════════════════════════╝"
    echo ""

    local prev_error=""
    phase_completed=false
    phase_iterations=0
    local loop_start_ts
    loop_start_ts=$(date +%s)

    for i in $(seq 1 "$max_iter"); do
        phase_iterations=$i
        local iter_start_ts
        iter_start_ts=$(date +%s)

        echo "┌──────────────────────────────────────────────────────────"
        echo "│ Iteration $i/$max_iter  [$(date +%H:%M:%S)]"
        echo "└──────────────────────────────────────────────────────────"

        # Pre-iteration snapshot
        local pre_src_count pre_test_count pre_test_result
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
        local prompt
        prompt=$(generate_execution_prompt "$ws_name" "$task_name" "$prev_error")

        # Run agent (fresh context each time)
        local local_timestamp output_file output
        local_timestamp=$(date +%Y%m%d-%H%M%S)
        output_file="$exec_dir/iteration-${task_label}-${i}-${local_timestamp}.md"
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

        local agent_elapsed
        agent_elapsed=$(elapsed_since "$iter_start_ts")
        log_info "Agent finished in $agent_elapsed"

        # Save iteration output
        echo "$output" > "$output_file"
        log_info "Output: $output_file"

        # Post-iteration delta
        local post_src_count post_test_count src_delta test_delta
        post_src_count=$(count_src_files)
        post_test_count=$(count_test_files)
        src_delta=$((post_src_count - pre_src_count))
        test_delta=$((post_test_count - pre_test_count))

        # Show what changed
        local git_changes new_files
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
        local validate_output=""
        if [[ -f "$SCRIPT_DIR/validate.sh" ]]; then
            validate_output=$("$SCRIPT_DIR/validate.sh" "$ws_name" ${task_name:+"$task_name"} 2>&1) || true

            # Extract and display key validation metrics
            local pass_line pending_line fail_line
            pass_line=$(echo "$validate_output" | grep "PASS:" | head -1 || true)
            pending_line=$(echo "$validate_output" | grep "PENDING:" || true)
            fail_line=$(echo "$validate_output" | grep "FAIL:" | head -1 || true)

            # Show test results
            local post_test_result
            post_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || true)
            [[ -n "$post_test_result" ]] && log_info "Tests: $post_test_result"

            # Show VALIDATION.md status
            [[ -n "$pass_line" ]] && log_info "VALIDATION.md — $pass_line"
            [[ -n "$pending_line" ]] && log_info "VALIDATION.md — $pending_line"
            [[ -n "$fail_line" ]] && log_info "VALIDATION.md — $fail_line"

            local iter_elapsed total_elapsed
            iter_elapsed=$(elapsed_since "$iter_start_ts")
            total_elapsed=$(elapsed_since "$loop_start_ts")

            echo ""
            if echo "$validate_output" | grep -q "^Recommendation: READY$"; then
                echo "  ✓ Iteration $i PASSED  [$iter_elapsed iter / $total_elapsed total]"
                log_info "Validation PASSED on iteration $i"
                phase_completed=true
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
            if echo "$output" | grep -q "^COMPLETE$"; then
                phase_completed=true
                break
            fi
        fi

        echo ""
    done

    phase_elapsed=$(elapsed_since "$loop_start_ts")
}

# --- Main execution ---

total_tasks=${#task_names[@]}
run_start_ts=$(date +%s)
timestamp_start=$(date +%Y%m%d-%H%M%S)
completed_tasks=()
failed_task=""

if [[ $total_tasks -gt 1 ]]; then
    echo ""
    echo "╔══════════════════════════════════════════════════════════╗"
    echo "║  ws-execute: $ws_name"
    echo "║  Phases: ${task_names[*]}"
    echo "║  Agent: $agent | Max iterations per phase: $max_iter"
    echo "╚══════════════════════════════════════════════════════════╝"
fi

for task_idx in $(seq 0 $((total_tasks - 1))); do
    task_name="${task_names[$task_idx]}"
    task_label="${task_name:-(workstream)}"
    phase_num=$((task_idx + 1))

    if [[ $total_tasks -gt 1 ]]; then
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "  Phase $phase_num/$total_tasks: $task_label"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    fi

    run_task "$task_name"

    if $phase_completed; then
        completed_tasks+=("$task_label")
        if [[ $total_tasks -gt 1 ]]; then
            echo ""
            echo "  ✓ Phase $phase_num/$total_tasks ($task_label) COMPLETE  [$phase_elapsed]"
        fi
    else
        failed_task="$task_label"
        if [[ $total_tasks -gt 1 ]]; then
            echo ""
            echo "  ✗ Phase $phase_num/$total_tasks ($task_label) FAILED after $phase_iterations iterations  [$phase_elapsed]"
            log_warn "Stopping — phase $task_label did not pass"
        fi
        break
    fi
done

# --- Final summary ---

total_elapsed=$(elapsed_since "$run_start_ts")
final_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || echo "no tests")
all_completed=$([[ -z "$failed_task" ]] && echo true || echo false)

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
if $all_completed; then
    echo "║  COMPLETE — $ws_name"
else
    echo "║  INCOMPLETE — $ws_name"
fi
if [[ $total_tasks -gt 1 ]]; then
    echo "║  Phases: ${#completed_tasks[@]}/$total_tasks complete"
    [[ ${#completed_tasks[@]} -gt 0 ]] && echo "║  Passed: ${completed_tasks[*]}"
    [[ -n "$failed_task" ]] && echo "║  Failed: $failed_task"
fi
echo "║  Time: $total_elapsed"
echo "║  $final_test_result"
echo "╚══════════════════════════════════════════════════════════╝"

# Write execution summary
summary_file="$exec_dir/execution-summary-${timestamp_start}.md"
cat > "$summary_file" << EOF
# Execution Summary

- **Workstream:** $ws_name
- **Tasks:** ${task_names[*]:-"(none)"}
- **Agent:** $agent
- **Max iterations per phase:** $max_iter
- **Phases completed:** ${#completed_tasks[@]}/$total_tasks
- **Status:** $(if $all_completed; then echo "COMPLETE"; else echo "INCOMPLETE — failed on $failed_task"; fi)
- **Total time:** $total_elapsed
- **Started:** $timestamp_start
- **Final tests:** $final_test_result

## Phase Results

$(for t in "${completed_tasks[@]}"; do
    echo "- ✓ $t"
done
if [[ -n "$failed_task" ]]; then
    echo "- ✗ $failed_task"
fi)
EOF

echo ""
if $all_completed; then
    log_info "All phases complete"
    log_info "Next: /ws-review $ws_name"
else
    log_warn "Stopped at phase: $failed_task"
    log_info "Re-run failed phase: ./scripts/ws-execute.sh $ws_name $failed_task --max-iter $max_iter"
fi
log_info "Summary: $summary_file"
