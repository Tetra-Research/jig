#!/bin/bash
# ws-review-cycle.sh — Full review cycle: review → synthesize → fix, repeated N rounds
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-review-cycle <workstream> [task] [--rounds 3] [--max-iter 3] [--agent claude|codex|both]"
    echo ""
    echo "Runs the full review pipeline in a loop:"
    echo "  1. Dual-agent code review (Claude + Codex)"
    echo "  2. Synthesize findings"
    echo "  3. Fix code (up to --max-iter per round)"
    echo "  4. Repeat for --rounds total (stops early if clean)"
    echo ""
    echo "Options:"
    echo "  --rounds       Number of review cycles (default: 3)"
    echo "  --max-iter     Max fix iterations per round (default: 3)"
    echo "  --agent        Review agents (default: both)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
task_name=""
rounds=3
max_iter=3
agent="both"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rounds) rounds="$2"; shift 2 ;;
        --max-iter) max_iter="$2"; shift 2 ;;
        --agent) agent="$2"; shift 2 ;;
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
mkdir -p "$review_dir"

if [[ ! -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' not found at $ws_dir"
    exit 1
fi

# --- Helpers ---

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

# Count findings by severity — counts list items (^- ) between section headers
count_findings() {
    local synth_file="$1"
    local critical major minor
    critical=$(sed -n '/\*\*Critical\*\*/,/\*\*Major\*\*/p' "$synth_file" | grep -c "^- " || true)
    major=$(sed -n '/\*\*Major\*\*/,/\*\*Minor\*\*/p' "$synth_file" | grep -c "^- " || true)
    minor=$(sed -n '/\*\*Minor\*\*/,/\*\*Strengths\*\*/p' "$synth_file" | grep -c "^- " || true)
    echo "$critical $major $minor"
}

# Check if synthesized review has no Critical or Major findings
# Uses actual list item counts, not keyword grep
review_is_clean() {
    local synth_file="$1"
    local counts
    counts=$(count_findings "$synth_file")
    local critical major
    critical=$(echo "$counts" | awk '{print $1}')
    major=$(echo "$counts" | awk '{print $2}')
    [[ "$critical" -eq 0 && "$major" -eq 0 ]]
}

# Format findings for display
format_findings() {
    local counts="$1"
    local critical major minor
    critical=$(echo "$counts" | awk '{print $1}')
    major=$(echo "$counts" | awk '{print $2}')
    minor=$(echo "$counts" | awk '{print $3}')
    echo "${critical}C/${major}M/${minor}m"
}

# --- Main cycle ---

cycle_start_ts=$(date +%s)
timestamp_start=$(date +%Y%m%d-%H%M%S)
completed_rounds=0
clean_review=false

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ws-review-cycle: $ws_name${task_name:+ / $task_name}"
echo "║  Rounds: $rounds | Fix iterations per round: $max_iter"
echo "║  Review agents: $agent"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

for round in $(seq 1 "$rounds"); do
    round_start_ts=$(date +%s)
    timestamp=$(date +%Y%m%d-%H%M%S)

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Round $round/$rounds  [$(date +%H:%M:%S)]"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # --- Step 1: Dual-agent review ---

    echo ""
    log_info "Step 1: Running code review..."

    claude_output="$review_dir/code-review-claude-${timestamp}.md"
    codex_output="$review_dir/code-review-codex-${timestamp}.md"

    prompt=$(generate_code_review_prompt "$ws_name" "$task_name")

    claude_pid=""
    codex_pid=""

    if [[ "$agent" == "both" || "$agent" == "claude" ]]; then
        log_info "  Starting Claude review..."
        (cd "$repo_root" && claude -p "$prompt" --output-format json) > "$claude_output" 2>&1 &
        claude_pid=$!
    fi

    if [[ "$agent" == "both" || "$agent" == "codex" ]]; then
        log_info "  Starting Codex review..."
        codex exec --json --full-auto "$prompt" > "$codex_output" 2>&1 &
        codex_pid=$!
    fi

    if [[ -n "$claude_pid" ]]; then
        wait "$claude_pid" || log_warn "Claude review exited non-zero"
    fi
    if [[ -n "$codex_pid" ]]; then
        wait "$codex_pid" || log_warn "Codex review exited non-zero"
    fi

    review_elapsed=$(elapsed_since "$round_start_ts")
    log_info "  Reviews complete [$review_elapsed]"

    # --- Step 2: Synthesize ---

    synth_output=""
    if [[ -f "$claude_output" && -f "$codex_output" ]]; then
        echo ""
        log_info "Step 2: Synthesizing findings..."

        synth_prompt=$(generate_code_review_synthesis_prompt "$claude_output" "$codex_output")
        synth_output="$review_dir/code-review-synthesized-${timestamp}.md"

        (cd "$repo_root" && claude -p "$synth_prompt" --output-format json) > "$synth_output" 2>&1 || {
            log_warn "Synthesis failed"
        }

        ln -sf "code-review-synthesized-${timestamp}.md" "$review_dir/code-review-synthesized.md"

        synth_elapsed=$(elapsed_since "$round_start_ts")
        log_info "  Synthesized [$synth_elapsed]"
    elif [[ -f "$claude_output" ]]; then
        # Single agent, use claude output directly
        synth_output="$claude_output"
        log_info "Step 2: Single agent — using Claude review as findings"
    elif [[ -f "$codex_output" ]]; then
        synth_output="$codex_output"
        log_info "Step 2: Single agent — using Codex review as findings"
    fi

    if [[ -z "$synth_output" || ! -f "$synth_output" ]]; then
        log_error "No review output produced in round $round"
        break
    fi

    # Show findings summary
    findings_counts=$(count_findings "$synth_output")
    findings_display=$(format_findings "$findings_counts")
    log_info "  Findings: $findings_display"

    # Check if clean
    if review_is_clean "$synth_output"; then
        echo ""
        echo "  ✓ Round $round: Clean review — no Critical or Major findings"
        clean_review=true
        completed_rounds=$round
        break
    fi

    # --- Step 3: Fix ---

    echo ""
    log_info "Step 3: Fixing findings..."

    fix_prompt=$(generate_review_fix_prompt "$ws_name" "$task_name" "$synth_output")
    prev_error=""
    fix_passed=false

    for fix_iter in $(seq 1 "$max_iter"); do
        fix_start_ts=$(date +%s)

        log_info "  Fix iteration $fix_iter/$max_iter..."

        if [[ $fix_iter -gt 1 && -n "$prev_error" ]]; then
            fix_prompt=$(generate_review_fix_prompt "$ws_name" "$task_name" "$synth_output")
            fix_prompt+="\n\n## Previous Fix Attempt Failed\n\n$prev_error\n\nFix these remaining issues."
        fi

        fix_output_file="$exec_dir/review-fix-round${round}-iter${fix_iter}-${timestamp}.md"
        fix_output=""

        case "${agent%%|*}" in
            both|claude)
                fix_output=$(cd "$repo_root" && claude -p "$fix_prompt" --permission-mode bypassPermissions --output-format json 2>&1) || true
                ;;
            codex)
                fix_output=$(codex exec --full-auto "$fix_prompt" 2>&1) || true
                ;;
        esac

        echo "$fix_output" > "$fix_output_file"

        fix_elapsed=$(elapsed_since "$fix_start_ts")
        log_info "  Agent finished [$fix_elapsed]"

        # Validate
        if [[ -f "$SCRIPT_DIR/validate.sh" ]]; then
            validate_output=$("$SCRIPT_DIR/validate.sh" "$ws_name" ${task_name:+"$task_name"} 2>&1) || true

            post_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || true)
            [[ -n "$post_test_result" ]] && log_info "  Tests: $post_test_result"

            if echo "$validate_output" | grep -q "^Recommendation: READY$"; then
                log_info "  Fix iteration $fix_iter passed"
                fix_passed=true
                break
            else
                log_warn "  Fix iteration $fix_iter — validation failed"
                prev_error=$(capture_validation_failures "$validate_output")
            fi
        else
            fix_passed=true
            break
        fi
    done

    round_elapsed=$(elapsed_since "$round_start_ts")
    completed_rounds=$round

    echo ""
    if $fix_passed; then
        echo "  ✓ Round $round complete — fixes applied [$round_elapsed]"
    else
        echo "  ✗ Round $round — fixes did not pass validation [$round_elapsed]"
        log_warn "Stopping — fix iterations exhausted"
        break
    fi
done

# --- Final summary ---

total_elapsed=$(elapsed_since "$cycle_start_ts")
final_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || echo "no tests")

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
if $clean_review; then
    echo "║  CLEAN — $ws_name${task_name:+ / $task_name}"
    echo "║  Clean review after $completed_rounds round(s)"
elif [[ $completed_rounds -eq $rounds ]]; then
    echo "║  COMPLETE — $ws_name${task_name:+ / $task_name}"
    echo "║  All $rounds rounds completed (may still have minor findings)"
else
    echo "║  INCOMPLETE — $ws_name${task_name:+ / $task_name}"
    echo "║  Stopped at round $completed_rounds/$rounds"
fi
echo "║  Time: $total_elapsed"
echo "║  $final_test_result"
echo "╚══════════════════════════════════════════════════════════╝"

# Write summary
summary_file="$exec_dir/review-cycle-summary-${timestamp_start}.md"
cat > "$summary_file" << EOF
# Review Cycle Summary

- **Workstream:** $ws_name
- **Task:** ${task_name:-"(none)"}
- **Rounds:** $completed_rounds/$rounds
- **Fix iterations per round:** $max_iter
- **Clean review:** $clean_review
- **Total time:** $total_elapsed
- **Started:** $timestamp_start
- **Final tests:** $final_test_result

## Rounds

$(for r in $(seq 1 "$completed_rounds"); do
    echo "### Round $r"
    review_file=$(ls -t "$review_dir"/code-review-synthesized-*.md 2>/dev/null | sed -n "${r}p" || true)
    [[ -n "$review_file" ]] && echo "- Review: $(basename "$review_file")"
    fix_files=$(ls "$exec_dir"/review-fix-round${r}-*.md 2>/dev/null || true)
    if [[ -n "$fix_files" ]]; then
        echo "$fix_files" | while read -r f; do echo "- Fix: $(basename "$f")"; done
    fi
    echo ""
done)
EOF

echo ""
if $clean_review; then
    log_info "Code is clean — ready for consolidation"
    log_info "Next: /ws-consolidate $ws_name"
else
    log_info "Review findings in: $review_dir/"
    log_info "Fix outputs in: $exec_dir/"
fi
log_info "Summary: $summary_file"
