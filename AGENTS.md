# Mentat

Spec-driven development workflow with dual-agent planning and iterative execution.

## Key Documents

- [PRD.md](PRD.md) - Product requirements document
- [docs/INVARIANTS.md](docs/INVARIANTS.md) - Project-wide constraints
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [docs/development/](docs/development/) - Methodology and philosophy

## Workflow

1. `/ws-init <name>` - Initialize workstream docs
2. `/ws-plan <name>` - Dual-agent planning (Claude + Codex), synthesize
3. `/ws-plan-review <name>` - Adversarial review of plan/specs
4. `/ws-execute <name> [task]` - Iterative execution with fresh-context retries
5. `/ws-validate` - Check task readiness (tests + VALIDATION.md)
6. `/ws-review <name> [task]` - Adversarial code review
7. `/ws-consolidate` - Capture learnings, update durable docs

## Conventions

- EARS format for all acceptance criteria in SPEC.md
- PLAN.md is source of truth for status/phases
- SPEC.md is canonical for requirements and interfaces
