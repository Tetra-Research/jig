#!/bin/bash
# ws-plan-review.sh — Dual-agent adversarial review of planning docs
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-plan-review <workstream> [--synthesize] [--agent claude|codex|both]"
    echo ""
    echo "Runs adversarial review of planning docs (SPEC, PLAN, SHARED-CONTEXT, etc.)"
    echo "Saves timestamped outputs to docs/workstreams/<name>/reviews/"
    echo ""
    echo "Options:"
    echo "  --synthesize   Merge both reviews into unified feedback"
    echo "  --agent        Run only one agent (default: both)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
agent="both"
synthesize=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --synthesize) synthesize=true; shift ;;
        --agent) agent="$2"; shift 2 ;;
        *) usage ;;
    esac
done

ws_dir="$(get_workstream_dir "$ws_name")"
repo_root="$(get_repo_root)"
review_dir="$ws_dir/reviews"
mkdir -p "$review_dir"
timestamp=$(date +%Y%m%d-%H%M%S)

if [[ ! -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' not found at $ws_dir"
    exit 1
fi

# Build review prompt
prompt=$(generate_plan_review_prompt "$ws_name")

claude_output="$review_dir/plan-review-claude-${timestamp}.md"
codex_output="$review_dir/plan-review-codex-${timestamp}.md"

claude_pid=""
codex_pid=""

# Run agents
if [[ "$agent" == "both" || "$agent" == "claude" ]]; then
    log_info "Starting Claude plan review..."
    (cd "$repo_root" && claude -p "$prompt" --output-format json) > "$claude_output" 2>&1 &
    claude_pid=$!
fi

if [[ "$agent" == "both" || "$agent" == "codex" ]]; then
    log_info "Starting Codex plan review..."
    codex exec --json --full-auto "$prompt" > "$codex_output" 2>&1 &
    codex_pid=$!
fi

# Wait for both
exit_code=0
if [[ -n "$claude_pid" ]]; then
    wait "$claude_pid" || { log_warn "Claude exited with non-zero status"; exit_code=1; }
    log_info "Claude review: $claude_output"
fi
if [[ -n "$codex_pid" ]]; then
    wait "$codex_pid" || { log_warn "Codex exited with non-zero status"; exit_code=1; }
    log_info "Codex review: $codex_output"
fi

# Synthesize if requested
if [[ "$synthesize" == true && -f "$claude_output" && -f "$codex_output" ]]; then
    log_info "Synthesizing reviews..."
    synth_prompt=$(generate_review_synthesis_prompt "$claude_output" "$codex_output")
    synth_output="$review_dir/plan-review-synthesized-${timestamp}.md"

    (cd "$repo_root" && claude -p "$synth_prompt" --output-format json) > "$synth_output" 2>&1 || {
        log_warn "Synthesis failed"
        exit_code=1
    }

    ln -sf "plan-review-synthesized-${timestamp}.md" "$review_dir/plan-review-synthesized.md"
    log_info "Synthesized review: $synth_output"
fi

echo ""
log_info "Plan review complete for '$ws_name'"
log_info "Outputs in: $review_dir/"
[[ -f "$claude_output" ]] && log_info "  Claude: plan-review-claude-${timestamp}.md"
[[ -f "$codex_output" ]] && log_info "  Codex:  plan-review-codex-${timestamp}.md"
[[ "$synthesize" == true ]] && log_info "  Merged: plan-review-synthesized-${timestamp}.md"
echo ""
log_info "Next steps:"
log_info "  1. Review findings from both agents"
log_info "  2. Fix critical/major issues in SPEC.md and PLAN.md"
log_info "  3. /ws-plan-review $ws_name again after fixes"

exit $exit_code
