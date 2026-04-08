# Django Pattern Playbook (Head-to-Head Skill Experiments)

Date: 2026-04-08
Purpose: capture the replicated standards and concrete control-vs-jig comparisons for the new head-to-head runner.

## Source Set (Replicated Locally)

### Django patterns
- backwards-compatible migrations
- indexing and unique constraints
- custom QuerySet/Manager patterns
- request validation and schema migration
- permissions and authorization checks
- model relation access and cache safety
- structured logging patterns
- mocked event service boundaries

### Testing patterns
- unit testing standards
- test organization and naming conventions
- fixtures and deterministic test data
- mocking boundaries and autospec guidance
- flakiness prevention checklist

### Process patterns
- migration PR strategy
- review checklists for architecture and safety

## High-Signal Skill Targets

1. Migration safety and backwards compatibility.
- two-step field removal and deprecation
- safe defaults for new `NOT NULL` fields
- avoid risky migration backfills in deploy paths
- explicit rule bypasses only when justified

2. Data-access and query discipline.
- views avoid domain writes
- services own writes/transactions
- selectors and QuerySet/Manager structures for reads
- relation-cache-safe access to avoid N+1 drift

3. Request validation and permission consistency.
- typed request contracts at view boundaries
- standardized auth/permission checks
- test patterns that verify permission behavior explicitly

4. Observability consistency.
- structured logging names (`method`, `method.step`)
- context binding for request and entity ids
- stable keys to support downstream log tooling

5. Deterministic test architecture.
- one behavior per test
- clear `# Act` phase in tests
- stable time/randomness setup
- mocks at system boundaries only

## Common LLM Failure Modes to Measure

- unsafe migrations that work locally but fail rolling deploy assumptions
- domain logic leaking into views
- ad hoc queries and N+1 regressions
- missing validation/permission wrappers
- non-deterministic tests and over-mocked internals
- inconsistent, low-signal logging

## First-Wave Comparison Pairs

1. `schema-migration-safety`
- Control: concise checklist and examples
- Jig: workflow/template scaffolding for safe migration changes

2. `view-contract-enforcer`
- Control: checklist for request validation + service handoff + response shape
- Jig: skeleton generator for validator, typed input, service call, response

3. `query-layer-discipline`
- Control: selector/queryset manager guidance
- Jig: scaffold for chainable QuerySet/Manager + selector entrypoints

4. `deterministic-service-test`
- Control: deterministic testing checklist
- Jig: test template with fixtures, boundary mocks, deterministic time/data, and `# Act`

5. `structured-logging-contract`
- Control: logging conventions checklist
- Jig: method/step logging skeleton with context binding

## Secondary Comparison Pairs

- `permissions-test-patterns`
- `read-replica-safety-pattern`
- `transaction-lock-discipline`
- `relation-cache-safe-access`
- `index-and-constraint-rollout-safety`

## Measurement Rules for Head-to-Head

- Keep scenario prompt and codebase identical between arms.
- Change only profile/skill implementation.
- Run both arms with thinking mode enabled.
- Score positive assertions and strict negative assertions.
- Capture and compare:
- pass rate
- score delta
- input/output/context tokens
- cache token components
- total tokens used
- cost and duration
- tool-call mix and jig invocation behavior

## Reusable Negative Assertions

- no `unittest.TestCase` for pytest-style tasks
- no `@patch` without `autospec=True`
- no non-deterministic time/random usage in deterministic tests
- no domain writes in view layer
- no unsafe migration constructs for migration tasks

## Cleanup Rule

- Do not include external project names in scenario docs, skill docs, prompts, or runner docs.
- Keep inspiration encoded as local pattern guidance only.
