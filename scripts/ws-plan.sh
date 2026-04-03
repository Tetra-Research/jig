#!/bin/bash
# ws-plan.sh — Run dual-agent planning (Claude + Codex) and optionally synthesize
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-plan <workstream> [--synthesize] [--prompt-file <path>] [--agent claude|codex|both]"
    echo ""
    echo "Runs both Claude and Codex against the same planning prompt in parallel."
    echo "Saves timestamped outputs to docs/workstreams/<name>/exec/"
    echo ""
    echo "Options:"
    echo "  --synthesize       Run synthesis to merge both plans"
    echo "  --prompt-file      Use a custom prompt file instead of auto-generating"
    echo "  --agent            Run only one agent (default: both)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
synthesize=false
prompt_file=""
agent="both"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --synthesize) synthesize=true; shift ;;
        --prompt-file) prompt_file="$2"; shift 2 ;;
        --agent) agent="$2"; shift 2 ;;
        *) usage ;;
    esac
done

ws_dir="$(get_workstream_dir "$ws_name")"
exec_dir="$(get_exec_dir "$ws_name")"
repo_root="$(get_repo_root)"
timestamp=$(date +%Y%m%d-%H%M%S)

if [[ ! -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' not found at $ws_dir"
    log_error "Run: just init $ws_name"
    exit 1
fi

# Build or load prompt
if [[ -n "$prompt_file" ]]; then
    prompt=$(cat "$prompt_file")
else
    prompt=$(generate_dual_plan_prompt "$ws_name")
fi

claude_output="$exec_dir/claude-plan-${timestamp}.md"
codex_output="$exec_dir/codex-plan-${timestamp}.md"

claude_pid=""
codex_pid=""

# Run agents
if [[ "$agent" == "both" || "$agent" == "claude" ]]; then
    log_info "Starting Claude planning..."
    claude -p "$prompt" --output-format json --cwd "$repo_root" > "$claude_output" 2>&1 &
    claude_pid=$!
fi

if [[ "$agent" == "both" || "$agent" == "codex" ]]; then
    log_info "Starting Codex planning..."
    codex exec --json --full-auto "$prompt" > "$codex_output" 2>&1 &
    codex_pid=$!
fi

# Wait for both
exit_code=0
if [[ -n "$claude_pid" ]]; then
    wait "$claude_pid" || { log_warn "Claude exited with non-zero status"; exit_code=1; }
    log_info "Claude output: $claude_output"
fi
if [[ -n "$codex_pid" ]]; then
    wait "$codex_pid" || { log_warn "Codex exited with non-zero status"; exit_code=1; }
    log_info "Codex output: $codex_output"
fi

# Synthesize if requested
if [[ "$synthesize" == true && -f "$claude_output" && -f "$codex_output" ]]; then
    log_info "Synthesizing plans..."
    synth_prompt=$(generate_synthesis_prompt "$claude_output" "$codex_output")
    synth_output="$exec_dir/synthesized-${timestamp}.md"

    claude -p "$synth_prompt" --output-format json --cwd "$repo_root" > "$synth_output" 2>&1 || {
        log_warn "Synthesis failed"
        exit_code=1
    }

    # Also create a symlink for easy access
    ln -sf "synthesized-${timestamp}.md" "$exec_dir/synthesized.md"
    log_info "Synthesized plan: $synth_output"
fi

echo ""
log_info "Planning complete for '$ws_name'"
log_info "Outputs in: $exec_dir/"
[[ -f "$claude_output" ]] && log_info "  Claude: claude-plan-${timestamp}.md"
[[ -f "$codex_output" ]] && log_info "  Codex:  codex-plan-${timestamp}.md"
[[ "$synthesize" == true ]] && log_info "  Merged: synthesized-${timestamp}.md"
echo ""
log_info "Next steps:"
log_info "  1. Review both plans"
log_info "  2. Run with --synthesize if you haven't"
log_info "  3. Edit exec/synthesized.md with your decisions"
log_info "  4. /ws-plan-review $ws_name"

exit $exit_code
