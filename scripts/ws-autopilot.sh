#!/bin/bash
# ws-autopilot.sh — Full pipeline: execute → review-cycle → consolidate → PR
# Run this and walk away. Come back to a PR.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-autopilot <workstream> [task...] [options]"
    echo ""
    echo "Runs the full pipeline end-to-end and opens a PR when done:"
    echo "  1. Execute tasks (ws-execute)"
    echo "  2. Review cycle (ws-review-cycle)"
    echo "  3. Consolidate learnings (ws-consolidate)"
    echo "  4. Commit + open PR"
    echo ""
    echo "Options:"
    echo "  --max-iter      Max fix iterations per step (default: 3)"
    echo "  --rounds        Review cycle rounds (default: 2)"
    echo "  --agent         Agent for execution (default: claude)"
    echo "  --review-agent  Agent for reviews (default: both)"
    echo "  --branch        Branch name (default: auto-generated)"
    echo "  --skip-execute  Skip execution, start at review"
    echo "  --skip-review   Skip review cycle"
    echo "  --base          Base branch for PR (default: main)"
    echo ""
    echo "Examples:"
    echo "  ws-autopilot core-engine phase-6 phase-7 --max-iter 3"
    echo "  ws-autopilot core-engine --skip-execute --rounds 3"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
task_names=()
max_iter=3
rounds=2
agent="claude"
review_agent="both"
branch=""
base_branch="main"
skip_execute=false
skip_review=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --max-iter) max_iter="$2"; shift 2 ;;
        --rounds) rounds="$2"; shift 2 ;;
        --agent) agent="$2"; shift 2 ;;
        --review-agent) review_agent="$2"; shift 2 ;;
        --branch) branch="$2"; shift 2 ;;
        --base) base_branch="$2"; shift 2 ;;
        --skip-execute) skip_execute=true; shift ;;
        --skip-review) skip_review=true; shift ;;
        --help|-h) usage ;;
        *) task_names+=("$1"); shift ;;
    esac
done

repo_root="$(get_repo_root)"
ws_dir="$(get_workstream_dir "$ws_name")"

if [[ ! -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' not found at $ws_dir"
    exit 1
fi

# --- Generate branch name ---

if [[ -z "$branch" ]]; then
    if [[ ${#task_names[@]} -gt 0 ]]; then
        branch="autopilot/${ws_name}/${task_names[0]}"
        [[ ${#task_names[@]} -gt 1 ]] && branch+="-to-${task_names[-1]}"
    else
        branch="autopilot/${ws_name}/$(date +%Y%m%d-%H%M%S)"
    fi
fi

# --- Helpers ---

elapsed_since() {
    local start_ts="$1"
    local now_ts
    now_ts=$(date +%s)
    local diff=$((now_ts - start_ts))
    local hrs=$((diff / 3600))
    local mins=$(( (diff % 3600) / 60 ))
    local secs=$((diff % 60))
    if [[ $hrs -gt 0 ]]; then
        echo "${hrs}h ${mins}m ${secs}s"
    elif [[ $mins -gt 0 ]]; then
        echo "${mins}m ${secs}s"
    else
        echo "${secs}s"
    fi
}

stage_status=()
record_stage() {
    local name="$1"
    local status="$2"
    local elapsed="$3"
    stage_status+=("$name: $status [$elapsed]")
}

# --- Pre-flight checks ---

pipeline_start_ts=$(date +%s)
timestamp=$(date +%Y%m%d-%H%M%S)

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  ws-autopilot: $ws_name"
if [[ ${#task_names[@]} -gt 0 ]]; then
echo "║  Tasks: ${task_names[*]}"
fi
echo "║  Branch: $branch"
echo "║  Execute: $( $skip_execute && echo "skip" || echo "$agent, max-iter=$max_iter" )"
echo "║  Review:  $( $skip_review && echo "skip" || echo "$review_agent, rounds=$rounds" )"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check for clean working tree
if [[ -n "$(cd "$repo_root" && git status --porcelain)" ]]; then
    log_warn "Working tree is not clean. Stashing changes..."
    (cd "$repo_root" && git stash push -m "autopilot-pre-${timestamp}")
    stashed=true
else
    stashed=false
fi

# Create and switch to feature branch
(cd "$repo_root" && git checkout -b "$branch" "$base_branch" 2>/dev/null) || {
    log_info "Branch '$branch' already exists, switching to it"
    (cd "$repo_root" && git checkout "$branch")
}

log_info "On branch: $branch"

# --- Stage 1: Execute ---

if ! $skip_execute; then
    stage_start_ts=$(date +%s)
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STAGE 1: Execute  [$(date +%H:%M:%S)]"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    exec_args=("$ws_name")
    for t in "${task_names[@]}"; do
        exec_args+=("$t")
    done
    exec_args+=(--agent "$agent" --max-iter "$max_iter")

    if "$SCRIPT_DIR/ws-execute.sh" "${exec_args[@]}"; then
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Execute" "PASS" "$stage_elapsed"
        log_info "Execution complete [$stage_elapsed]"

        # Commit execution results
        (cd "$repo_root" && git add -A && git diff --cached --quiet) || {
            (cd "$repo_root" && git commit -m "feat($ws_name): implement ${task_names[*]:-all tasks}

Automated by ws-autopilot")
            log_info "Committed execution results"
        }
    else
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Execute" "FAIL" "$stage_elapsed"
        log_error "Execution failed [$stage_elapsed]"
        log_error "Stopping pipeline — fix execution failures before retrying"

        # Still commit partial work so it's not lost
        (cd "$repo_root" && git add -A && git diff --cached --quiet) || {
            (cd "$repo_root" && git commit -m "wip($ws_name): partial execution (failed)

Automated by ws-autopilot")
        }

        # Push and create draft PR for failed runs
        (cd "$repo_root" && git push -u origin "$branch" 2>/dev/null) || true
        pr_url=$(cd "$repo_root" && gh pr create \
            --title "WIP: $ws_name ${task_names[*]:-}" \
            --body "$(cat <<EOF
## Summary
- Autopilot execution **failed** — needs manual intervention
- Branch: \`$branch\`

## Stage Results
$(printf '%s\n' "${stage_status[@]}" | sed 's/^/- /')

🤖 Generated with ws-autopilot
EOF
)" --draft 2>/dev/null) || true

        echo ""
        echo "╔══════════════════════════════════════════════════════════════╗"
        echo "║  AUTOPILOT STOPPED — Execution failed"
        echo "║  Partial work committed to: $branch"
        [[ -n "${pr_url:-}" ]] && echo "║  Draft PR: $pr_url"
        echo "╚══════════════════════════════════════════════════════════════╝"
        exit 1
    fi
else
    log_info "Skipping execution (--skip-execute)"
    record_stage "Execute" "SKIP" "0s"
fi

# --- Stage 2: Review Cycle ---

if ! $skip_review; then
    stage_start_ts=$(date +%s)
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STAGE 2: Review Cycle  [$(date +%H:%M:%S)]"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    review_args=("$ws_name")
    # Pass first task for scoping if available
    [[ ${#task_names[@]} -gt 0 ]] && review_args+=("${task_names[0]}")
    review_args+=(--rounds "$rounds" --max-iter "$max_iter" --agent "$review_agent")

    if "$SCRIPT_DIR/ws-review-cycle.sh" "${review_args[@]}"; then
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Review" "PASS" "$stage_elapsed"
        log_info "Review cycle complete [$stage_elapsed]"
    else
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Review" "WARN" "$stage_elapsed"
        log_warn "Review cycle had issues [$stage_elapsed] — continuing"
    fi

    # Commit review fixes
    (cd "$repo_root" && git add -A && git diff --cached --quiet) || {
        (cd "$repo_root" && git commit -m "fix($ws_name): apply review findings

Automated by ws-autopilot review cycle")
        log_info "Committed review fixes"
    }
else
    log_info "Skipping review (--skip-review)"
    record_stage "Review" "SKIP" "0s"
fi

# --- Stage 3: Consolidate ---

stage_start_ts=$(date +%s)
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  STAGE 3: Consolidate  [$(date +%H:%M:%S)]"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if "$SCRIPT_DIR/ws-consolidate.sh" "$ws_name"; then
    stage_elapsed=$(elapsed_since "$stage_start_ts")
    record_stage "Consolidate" "PASS" "$stage_elapsed"
    log_info "Consolidation complete [$stage_elapsed]"
else
    stage_elapsed=$(elapsed_since "$stage_start_ts")
    record_stage "Consolidate" "WARN" "$stage_elapsed"
    log_warn "Consolidation had issues [$stage_elapsed] — continuing"
fi

# Commit consolidation changes
(cd "$repo_root" && git add -A && git diff --cached --quiet) || {
    (cd "$repo_root" && git commit -m "docs($ws_name): consolidate learnings

Automated by ws-autopilot consolidation")
    log_info "Committed consolidation"
}

# --- Stage 4: Push + PR ---

stage_start_ts=$(date +%s)
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  STAGE 4: Push + PR  [$(date +%H:%M:%S)]"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Get final test status
final_test_result=$(cd "$repo_root" && cargo test 2>&1 | grep "^test result:" | head -1 || echo "no tests")

# Push
(cd "$repo_root" && git push -u origin "$branch")

# Build PR body
task_list=""
if [[ ${#task_names[@]} -gt 0 ]]; then
    for t in "${task_names[@]}"; do
        task_list+="- $t\n"
    done
else
    task_list="- (all tasks)\n"
fi

pr_title="$ws_name: ${task_names[*]:-implementation}"

# Truncate title if too long
[[ ${#pr_title} -gt 70 ]] && pr_title="${pr_title:0:67}..."

pr_url=$(cd "$repo_root" && gh pr create \
    --base "$base_branch" \
    --title "$pr_title" \
    --body "$(cat <<EOF
## Summary
Automated implementation of workstream \`$ws_name\`.

**Tasks:**
$(echo -e "$task_list")
## Pipeline Results

$(printf '%s\n' "${stage_status[@]}" | sed 's/^/- /')

**Tests:** $final_test_result

## Pipeline Config
- Execute: agent=$agent, max-iter=$max_iter
- Review: agent=$review_agent, rounds=$rounds
- Total time: $(elapsed_since "$pipeline_start_ts")

## Test Plan
- [ ] Review code changes
- [ ] Verify test coverage
- [ ] Check for any remaining review findings in \`docs/workstreams/$ws_name/reviews/\`

🤖 Generated with ws-autopilot
EOF
)")

stage_elapsed=$(elapsed_since "$stage_start_ts")
record_stage "PR" "DONE" "$stage_elapsed"

# --- Final Summary ---

total_elapsed=$(elapsed_since "$pipeline_start_ts")

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  AUTOPILOT COMPLETE"
echo "║"
echo "║  Workstream: $ws_name"
if [[ ${#task_names[@]} -gt 0 ]]; then
echo "║  Tasks: ${task_names[*]}"
fi
echo "║  Branch: $branch"
echo "║  Time: $total_elapsed"
echo "║"
echo "║  Stages:"
for s in "${stage_status[@]}"; do
echo "║    $s"
done
echo "║"
echo "║  Tests: $final_test_result"
echo "║  PR: $pr_url"
echo "╚══════════════════════════════════════════════════════════════╝"

# Restore stash if we stashed
if $stashed; then
    log_info "Restoring stashed changes..."
    (cd "$repo_root" && git stash pop) || log_warn "Could not restore stash automatically"
fi
