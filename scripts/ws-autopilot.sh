#!/bin/bash
# ws-autopilot.sh — Full pipeline: init → plan → execute → review → consolidate → PR
# Run this and walk away. Come back to a PR.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-autopilot <workstream> [task...] [options]"
    echo ""
    echo "Runs the full pipeline end-to-end and opens a PR when done:"
    echo "  1. Init workstream (if needed)"
    echo "  2. Populate spec + plan (if templates)"
    echo "  3. Dual-agent planning + synthesis"
    echo "  4. Execute tasks (ws-execute)"
    echo "  5. Review cycle (ws-review-cycle)"
    echo "  6. Consolidate learnings (ws-consolidate)"
    echo "  7. Commit + open PR"
    echo ""
    echo "Options:"
    echo "  --max-iter      Max fix iterations per step (default: 3)"
    echo "  --rounds        Review cycle rounds (default: 2)"
    echo "  --agent         Agent for execution (default: claude)"
    echo "  --review-agent  Agent for reviews (default: both)"
    echo "  --plan-agent    Agent for planning (default: both)"
    echo "  --branch        Branch name (default: auto-generated)"
    echo "  --base          Base branch for PR (default: main)"
    echo "  --skip-init     Skip init + spec population"
    echo "  --skip-plan     Skip planning"
    echo "  --skip-execute  Skip execution, start at review"
    echo "  --skip-review   Skip review cycle"
    echo ""
    echo "Examples:"
    echo "  ws-autopilot replace-patch                              # new workstream, full pipeline"
    echo "  ws-autopilot core-engine phase-6 phase-7 --max-iter 3   # existing workstream, new phases"
    echo "  ws-autopilot core-engine --skip-init --skip-plan        # just execute + review"
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
plan_agent="both"
branch=""
base_branch="main"
skip_init=false
skip_plan=false
skip_execute=false
skip_review=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --max-iter) max_iter="$2"; shift 2 ;;
        --rounds) rounds="$2"; shift 2 ;;
        --agent) agent="$2"; shift 2 ;;
        --review-agent) review_agent="$2"; shift 2 ;;
        --plan-agent) plan_agent="$2"; shift 2 ;;
        --branch) branch="$2"; shift 2 ;;
        --base) base_branch="$2"; shift 2 ;;
        --skip-init) skip_init=true; shift ;;
        --skip-plan) skip_plan=true; shift ;;
        --skip-execute) skip_execute=true; shift ;;
        --skip-review) skip_review=true; shift ;;
        --help|-h) usage ;;
        *) task_names+=("$1"); shift ;;
    esac
done

repo_root="$(get_repo_root)"
ws_dir="$(get_workstream_dir "$ws_name")"

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

# Check if a file is still a template (has <!-- --> placeholders in key sections)
is_template() {
    local file="$1"
    [[ ! -f "$file" ]] && return 0
    # If it has 3+ HTML comment placeholders, it's still a template
    local placeholder_count
    placeholder_count=$(grep -c "<!-- " "$file" 2>/dev/null) || placeholder_count=0
    [[ "$placeholder_count" -ge 3 ]]
}

# --- Pre-flight ---

pipeline_start_ts=$(date +%s)
timestamp=$(date +%Y%m%d-%H%M%S)

# Safe string for task names (avoids unbound variable with set -u on empty arrays)
if [[ ${#task_names[@]} -gt 0 ]]; then
    task_names_str="${task_names[*]}"
else
    task_names_str=""
fi

# Auto-detect which stages to skip
ws_exists=false
[[ -d "$ws_dir" ]] && ws_exists=true

spec_populated=false
if $ws_exists && [[ -f "$ws_dir/SPEC.md" ]] && ! is_template "$ws_dir/SPEC.md"; then
    spec_populated=true
fi

plan_exists=false
if $ws_exists; then
    exec_dir="$(get_exec_dir "$ws_name")"
    if [[ -f "$exec_dir/synthesized.md" ]] || ls "$exec_dir"/synthesized-*.md &>/dev/null 2>&1; then
        plan_exists=true
    fi
fi

# Apply auto-detection to skip flags
if $ws_exists && $spec_populated; then
    skip_init=true
fi
if $plan_exists; then
    skip_plan=true
fi

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  ws-autopilot: $ws_name"
if [[ ${#task_names[@]} -gt 0 ]]; then
echo "║  Tasks: ${task_names[*]}"
fi
echo "║  Branch: $branch"
echo "║  Init:    $( $skip_init && echo "skip (exists)" || echo "create + populate" )"
echo "║  Plan:    $( $skip_plan && echo "skip (exists)" || echo "$plan_agent, synthesize" )"
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

# --- Stage 1: Init + Populate ---

if ! $skip_init; then
    stage_start_ts=$(date +%s)
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STAGE 1: Init + Populate Spec  [$(date +%H:%M:%S)]"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Init if needed
    if ! $ws_exists; then
        log_info "Initializing workstream '$ws_name'..."
        "$SCRIPT_DIR/ws-init.sh" "$ws_name"
        ws_dir="$(get_workstream_dir "$ws_name")"
    fi

    # Populate spec docs using an agent
    if ! $spec_populated; then
        log_info "Populating SPEC.md, PLAN.md, SHARED-CONTEXT.md, NARRATIVE.md..."

        populate_prompt=$(generate_spec_population_prompt "$ws_name")

        populate_output=$(cd "$repo_root" && claude -p "$populate_prompt" \
            --permission-mode bypassPermissions --output-format json 2>&1) || true

        # Save the agent output for reference
        exec_dir="$(get_exec_dir "$ws_name")"
        echo "$populate_output" > "$exec_dir/spec-population-${timestamp}.md"
        log_info "Population output: $exec_dir/spec-population-${timestamp}.md"
    fi

    stage_elapsed=$(elapsed_since "$stage_start_ts")

    # Verify population worked
    if is_template "$ws_dir/SPEC.md"; then
        record_stage "Init+Spec" "WARN" "$stage_elapsed"
        log_warn "SPEC.md may still be a template — planning may produce generic results"
    else
        record_stage "Init+Spec" "PASS" "$stage_elapsed"
        log_info "Spec populated [$stage_elapsed]"
    fi

    # Commit init + spec
    (cd "$repo_root" && git add -A && git diff --cached --quiet) || {
        (cd "$repo_root" && git commit -m "docs($ws_name): initialize workstream with spec and plan

Automated by ws-autopilot")
        log_info "Committed workstream docs"
    }
else
    log_info "Skipping init (workstream exists with populated spec)"
    record_stage "Init+Spec" "SKIP" "0s"
fi

# --- Stage 2: Plan ---

if ! $skip_plan; then
    stage_start_ts=$(date +%s)
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STAGE 2: Dual-Agent Planning  [$(date +%H:%M:%S)]"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if "$SCRIPT_DIR/ws-plan.sh" "$ws_name" --synthesize --agent "$plan_agent"; then
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Plan" "PASS" "$stage_elapsed"
        log_info "Planning complete [$stage_elapsed]"
    else
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Plan" "WARN" "$stage_elapsed"
        log_warn "Planning had issues [$stage_elapsed] — continuing with available plan"
    fi

    # Commit planning artifacts
    (cd "$repo_root" && git add -A && git diff --cached --quiet) || {
        (cd "$repo_root" && git commit -m "docs($ws_name): dual-agent execution plan

Automated by ws-autopilot planning")
        log_info "Committed planning artifacts"
    }
else
    log_info "Skipping planning (synthesized plan exists)"
    record_stage "Plan" "SKIP" "0s"
fi

# --- Stage 3: Execute ---

if ! $skip_execute; then
    stage_start_ts=$(date +%s)
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STAGE 3: Execute  [$(date +%H:%M:%S)]"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    exec_args=("$ws_name")
    if [[ ${#task_names[@]} -gt 0 ]]; then
        for t in "${task_names[@]}"; do
            exec_args+=("$t")
        done
    fi
    exec_args+=(--agent "$agent" --max-iter "$max_iter")

    if "$SCRIPT_DIR/ws-execute.sh" "${exec_args[@]}"; then
        stage_elapsed=$(elapsed_since "$stage_start_ts")
        record_stage "Execute" "PASS" "$stage_elapsed"
        log_info "Execution complete [$stage_elapsed]"

        # Commit execution results
        (cd "$repo_root" && git add -A && git diff --cached --quiet) || {
            (cd "$repo_root" && git commit -m "feat($ws_name): implement ${task_names_str:-all tasks}

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
            --title "WIP: $ws_name ${task_names_str}" \
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

# --- Stage 4: Review Cycle ---

if ! $skip_review; then
    stage_start_ts=$(date +%s)
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STAGE 4: Review Cycle  [$(date +%H:%M:%S)]"
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

# --- Stage 5: Consolidate ---

stage_start_ts=$(date +%s)
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  STAGE 5: Consolidate  [$(date +%H:%M:%S)]"
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

# --- Stage 6: Push + PR ---

stage_start_ts=$(date +%s)
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  STAGE 6: Push + PR  [$(date +%H:%M:%S)]"
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

pr_title="$ws_name: ${task_names_str:-implementation}"

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
- Plan: agent=$plan_agent
- Execute: agent=$agent, max-iter=$max_iter
- Review: agent=$review_agent, rounds=$rounds
- Total time: $(elapsed_since "$pipeline_start_ts")

## Test Plan
- [ ] Review spec + plan docs in \`docs/workstreams/$ws_name/\`
- [ ] Review code changes
- [ ] Verify test coverage
- [ ] Check review findings in \`docs/workstreams/$ws_name/reviews/\`

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
