# SDD Workflow Tooling

# Initialize a new workstream
init ws *args:
    ./scripts/ws-init.sh {{ws}} {{args}}

# Run dual-agent planning for a workstream
plan ws *args:
    ./scripts/ws-plan.sh {{ws}} {{args}}

# Execute a workstream task with agent iteration loop
execute ws *args:
    ./scripts/ws-execute.sh {{ws}} {{args}}

# Run validation checks
validate *args:
    ./scripts/validate.sh {{args}}

# Show full workflow command order
workflow:
    @echo "Workflow:"
    @echo "  1. just init <ws>           — Create workstream docs"
    @echo "  2. just plan <ws>           — Dual-agent planning + synthesis"
    @echo "  3. /ws-plan-review <ws>     — Adversarial plan review"
    @echo "  4. just execute <ws> [task] — Iterative execution loop"
    @echo "  5. just validate            — Check readiness"
    @echo "  6. /ws-review <ws> [task]   — Code review"
    @echo "  7. /ws-consolidate          — Capture learnings"
