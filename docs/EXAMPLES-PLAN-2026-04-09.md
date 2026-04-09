# Examples Plan

## Decision

The repo should add a first-class `examples/` directory at the root.

Examples should be the primary deeper-learning surface for `jig`. They should teach the product through concrete before/after patterns rather than through a large public docs system.

This plan assumes:

- the public site remains a single landing page
- the landing page includes a compact quick start
- deeper learning links users into repo examples

## Why Examples Matter

`jig` is best understood through repeatable code-shaping patterns:

- create a file with a deterministic structure
- inject a block into the right place
- replace a region safely
- patch existing code in a predictable way

That is easier to teach with self-contained examples than with long prose.

Examples are especially important because:

- the product is used by humans and agents
- selector semantics matter
- before/after state is central to understanding the value
- the strongest product story is routine, shape-constrained backend work

## Source of Initial Examples

The first example set should come from the current head-to-head eval scenarios.

These are the right seed set because they are:

- already representative of routine backend work
- already grounded in before/after code
- already constrained enough to be understandable
- already important to the current product story

The first five examples should be:

1. `add-service-test`
2. `query-layer-discipline`
3. `schema-migration-safety`
4. `structured-logging-contract`
5. `view-contract-enforcer`

These map to the current head-to-head patterns and are sufficient for the first public release.

## Repo Layout

Recommended layout:

```text
jig/
  examples/
    README.md
    add-service-test/
    query-layer-discipline/
    schema-migration-safety/
    structured-logging-contract/
    view-contract-enforcer/
```

Each example directory should be self-contained and understandable on its own.

## Example Directory Contract

Each example should use the same internal structure:

```text
examples/<example-name>/
  README.md
  recipe.yaml
  vars.json
  before/
  after/
  templates/
```

If an example eventually requires workflow composition, it may add:

```text
workflow.yaml
```

But workflows should not be the starting point unless they are clearly necessary. The first wave should stay recipe-first and easy to read.

## Example README Contract

Every example `README.md` should be short, direct, and runnable.

Each example README should include:

1. what problem the example solves
2. when to use this pattern
3. the exact `jig run` command
4. the expected file changes
5. where to look for before/after state

Recommended shape:

```md
# <example-name>

## What This Does
<one short explanation>

## When To Use It
<one short explanation>

## Run
```bash
jig run recipe.yaml --vars @vars.json
```

## Expected Changes
- updates `...`
- creates `...`

## Before / After
See `before/` and `after/`.
```

The README should not assume the reader has already studied internal docs.

## Example Quality Rules

Each example should demonstrate:

1. deterministic selector usage
2. minimal variable surface
3. clear and stable naming
4. readable before/after state
5. no unnecessary cleverness

Examples are teaching artifacts, not stress tests. They should bias toward clarity.

## Naming Conventions

Variable names should be consistent across examples wherever possible.

Prefer names like:

- `target_file`
- `function_name`
- `class_name`
- `model_name`
- `request_schema`
- `response_schema`
- `event_namespace`
- `step_name`

This consistency is important for:

- humans scanning examples quickly
- agents pattern-matching across examples
- later plugin and skill guidance

## Public vs Eval Boundary

Examples may reuse the good parts of the eval scenarios, but they must not look like eval fixtures.

Acceptable reuse:

- task shape
- before/after code
- recipe structure
- variables
- templates

What must be removed:

- scenario IDs
- benchmark framing
- control vs jig framing
- harness assertions
- experiment log language
- eval-specific routes or artifact naming

The examples should read like product examples, not test harness material.

## Initial Example Ordering

The top-level `examples/README.md` should present the examples in this order:

1. `add-service-test`
2. `structured-logging-contract`
3. `view-contract-enforcer`
4. `query-layer-discipline`
5. `schema-migration-safety`

Reasoning:

- the first two are easy to understand quickly
- the middle one shows request/response contract work
- the last two show stronger brownfield patching value

This order is better for teaching than ordering by eval chronology.

## Top-Level `examples/README.md`

The root `examples/README.md` should include:

1. a short explanation of what the examples are for
2. a note that each example is self-contained
3. an index of examples
4. a short note on common conventions
5. a pointer back to the landing page quick start and repo README

This file should be the gateway into the example set.

## Relationship to Public Site

The public site should not try to duplicate the examples fully.

Instead:

- the site should show a few curated example teasers
- each teaser should link into the relevant example directory in the repo

This keeps the landing page focused while preserving enough depth in the repo.

## Relationship to Claude and Codex Integrations

Examples should stay focused on `jig` itself.

They should not try to fully teach plugin or skill authoring. That should live in separate integration-specific materials.

However, examples should be compatible with those future integrations by being:

- clear
- self-contained
- stable in naming
- representative of when `jig` is useful

## What We Should Not Do

We should not:

- start with a large example catalog
- include toy examples that do not resemble real maintenance work
- expose raw eval artifacts directly
- make examples depend on the harness
- overload examples with plugin-specific instructions

The initial example set should stay focused and strong.

## Execution Plan

1. Create `examples/README.md`
2. Build `add-service-test`
3. Build `structured-logging-contract`
4. Build `view-contract-enforcer`
5. Build `query-layer-discipline`
6. Build `schema-migration-safety`
7. Add links from the landing page to the examples

## Acceptance Criteria

- The repo SHALL expose a root `examples/` directory.
- The initial example set SHALL contain five examples derived from the current routine backend patterns.
- Each example SHALL be self-contained.
- Each example SHALL include `README.md`, `recipe.yaml`, `vars.json`, `before/`, `after/`, and `templates/`.
- Examples SHALL not reference eval harness internals.
- The top-level `examples/README.md` SHALL provide a navigable index into the example set.
- The public site SHALL be able to link to the examples without requiring a separate public docs system.
