#!/bin/bash
# ws-review-fix.sh — Fix code based on review findings (iteration loop)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-review-fix <workstream> [task] [--findings <path>] [--agent claude|codex] [--max-iter 3]"
    echo ""
    echo "Feeds review findings to an agent to fix the code, then validates."
    echo ""
    echo "If --findings is not specified, uses the latest synthesized review."
    echo ""
    echo "Options:"
    echo "  --findings     Path to review findings file (default: latest synthesized)"
    echo "  --agent        Agent to use (default: claude)"
    echo "  --max-iter     Maximum fix iterations (default: 3)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
task_name=""
findings_file=""
agent="claude"
max_iter=3

while [[ $# -gt 0 ]]; do
    case "$1" in
        --findings) findings_file="$2"; shift 2 ;;
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
review_dir="$ws_dir/reviews"
exec_dir="$(get_exec_dir "$ws_name")"
repo_root="$(get_repo_root)"

if [[ ! -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' not found at $ws_dir"
    exit 1
fi

# Find findings file
if [[ -z "$findings_file" ]]; then
    # Try synthesized symlink first, then latest timestamped
    if [[ -f "$review_dir/code-review-synthesized.md" ]]; then
        findings_file="$review_dir/code-review-synthesized.md"
    else
        findings_file=$(ls -t "$review_dir"/code-review-synthesized-*.md 2>/dev/null | head -1 || true)
    fi

    if [[ -z "$findings_file" || ! -f "$findings_file" ]]; then
        # Fall back to latest claude review
        findings_file=$(ls -t "$review_dir"/code-review-claude-*.md 2>/dev/null | head -1 || true)
    fi

    if [[ -z "$findings_file" || ! -f "$findings_file" ]]; then
        log_error "No review findings found. Run /ws-review $ws_name --synthesize first."
        exit 1
    fi
fi

if [[ ! -f "$findings_file" ]]; then
    log_error "Findings file not found: $findings_file"
    exit 1
fi

log_info "Findings: $findings_file"

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

# --- Fix loop ---

timestamp_start=$(date +%Y%m%d-%H%M%S)
loop_start_ts=$(date +%s)
completed=false

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ws-review-fix: $ws_name${task_name:+ / $task_name}"
echo "║  Agent: $agent | Max iterations: $max_iter"
echo "║  Findings: $(basename "$findings_file")"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

prev_error=""

for i in $(seq 1 "$max_iter"); do
    iter_start_ts=$(date +%s)

    echo "┌──────────────────────────────────────────────────────────"
    echo "│ Fix iteration $i/$max_iter  [$(date +%H:%M:%S)]"
    echo "└──────────────────────────────────────────────────────────"

    # Build prompt — first iteration uses findings, subsequent use validation errors
    if [[ $i -eq 1 ]]; then
        prompt=$(generate_review_fix_prompt "$ws_name" "$task_name" "$findings_file")
    else
        # On retry, append the validation failures to the original findings
        prompt=$(generate_review_fix_prompt "$ws_name" "$task_name" "$findings_file")
        prompt+="\n\n## Previous Fix Attempt Failed\n\nThe following issues remain after the previous fix attempt:\n\n$prev_error\n\nFix these remaining issues."
    fi

    output_file="$exec_dir/review-fix-${i}-${timestamp_start}.md"
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

    echo "$output" > "$output_file"
    log_info "Output: $output_file"

    # Validate
    echo ""
    log_info "Running validation..."
    validate_output=""
    if [[ -f "$SCRIPT_DIR/validate.sh" ]]; then
        validate_output=$("$SCRIPT_DIR/validate.sh" "$ws_name" ${task_name:+"$task_name"} 2>&1) || true

        # Show test results
        post_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || true)
        [[ -n "$post_test_result" ]] && log_info "Tests: $post_test_result"

        iter_elapsed=$(elapsed_since "$iter_start_ts")
        total_elapsed=$(elapsed_since "$loop_start_ts")

        echo ""
        if echo "$validate_output" | grep -q "^Recommendation: READY$"; then
            echo "  ✓ Fix iteration $i PASSED  [$iter_elapsed iter / $total_elapsed total]"
            completed=true
            break
        else
            echo "  ✗ Fix iteration $i — tests not passing  [$iter_elapsed iter / $total_elapsed total]"
            prev_error=$(capture_validation_failures "$validate_output")
            if [[ $i -lt $max_iter ]]; then
                log_info "Feeding failures to iteration $((i + 1))..."
            fi
        fi
    else
        log_warn "No validate.sh found"
        completed=true
        break
    fi

    echo ""
done

# --- Summary ---

total_elapsed=$(elapsed_since "$loop_start_ts")
final_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || echo "no tests")

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
if $completed; then
    echo "║  FIXES APPLIED — $ws_name${task_name:+ / $task_name}"
else
    echo "║  FIXES INCOMPLETE — $ws_name${task_name:+ / $task_name}"
fi
echo "║  Iterations: $i/$max_iter | Time: $total_elapsed"
echo "║  $final_test_result"
echo "╚══════════════════════════════════════════════════════════╝"

echo ""
if $completed; then
    log_info "All fixes applied and validated"
    log_info "Next: review changes, then commit"
else
    log_warn "Some fixes could not be applied within $max_iter iterations"
    log_info "Review outputs in $exec_dir/"
fi
