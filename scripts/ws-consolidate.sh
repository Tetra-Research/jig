#!/bin/bash
# ws-consolidate.sh — Capture learnings and update durable docs
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-consolidate <workstream> [--agent claude|codex]"
    echo ""
    echo "Reviews recent changes and updates durable documentation."
    echo "Saves output to docs/workstreams/<name>/exec/"
    echo ""
    echo "Options:"
    echo "  --agent    Agent to use (default: claude)"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
agent="claude"

while [[ $# -gt 0 ]]; do
    case "$1" in
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
    exit 1
fi

# Build consolidation prompt
prompt=$(generate_consolidation_prompt "$ws_name")

output_file="$exec_dir/consolidation-${timestamp}.md"

log_info "Starting consolidation for '$ws_name' with $agent..."

case "$agent" in
    claude)
        (cd "$repo_root" && claude -p "$prompt" --permission-mode bypassPermissions --output-format json) > "$output_file" 2>&1 || {
            log_warn "$agent exited with non-zero status"
        }
        ;;
    codex)
        codex exec --json --full-auto "$prompt" > "$output_file" 2>&1 || {
            log_warn "$agent exited with non-zero status"
        }
        ;;
    *)
        log_error "Unknown agent: $agent (use claude or codex)"
        exit 1
        ;;
esac

log_info "Consolidation output: $output_file"
echo ""
log_info "Consolidation complete for '$ws_name'"
log_info "Output: $exec_dir/consolidation-${timestamp}.md"
echo ""
log_info "Next steps:"
log_info "  1. Review the consolidation output"
log_info "  2. Apply the suggested doc updates"
log_info "  3. Commit the updated docs"
