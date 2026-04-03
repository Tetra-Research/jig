#!/bin/bash
# ws-init.sh — Initialize workstream document structure
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

usage() {
    echo "Usage: ws-init <workstream-name> [--discovery]"
    echo ""
    echo "Creates workstream document structure under docs/workstreams/<name>/"
    echo ""
    echo "Options:"
    echo "  --discovery    Create only discovery/ folder for research phase"
    exit 1
}

[[ $# -lt 1 ]] && usage

ws_name="$1"
shift
discovery_only=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --discovery) discovery_only=true; shift ;;
        *) usage ;;
    esac
done

ws_dir="$(get_workstream_dir "$ws_name")"

if [[ -d "$ws_dir" ]]; then
    log_error "Workstream '$ws_name' already exists at $ws_dir"
    exit 1
fi

mkdir -p "$ws_dir"

if [[ "$discovery_only" == true ]]; then
    mkdir -p "$ws_dir/discovery"
    cat > "$ws_dir/discovery/README.md" << EOF
# Discovery: $ws_name

## Research Areas

- <!-- Area to explore -->

## Open Questions

- <!-- Questions to answer before planning -->
EOF
    log_info "Created discovery-only workstream at $ws_dir/discovery/"
else
    mkdir -p "$ws_dir/tasks"

    generate_plan_md "$ws_name" "$ws_dir"
    generate_spec_md "$ws_name" "$ws_dir"
    generate_shared_context_md "$ws_name" "$ws_dir"
    generate_narrative_md "$ws_name" "$ws_dir"

    log_info "Created workstream at $ws_dir/"
    log_info "  PLAN.md, SPEC.md, SHARED-CONTEXT.md, NARRATIVE.md"
    log_info ""
    log_info "Next: /ws-plan $ws_name"
fi
