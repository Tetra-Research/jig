# Site Design

## Decision

`jig` should ship with a small, self-contained public marketing site and a first-class `examples/` directory in the repo.

We should not build a separate public docs system yet.

The public surface should be:

1. one landing page on a subdomain
2. a compact quick-start section on that page
3. links out to repo examples
4. links out to Claude and Codex integrations

Internal engineering docs stay in [`docs/`](/Users/tylerobriant/code/tetra/jig/docs). Public site content should not reuse those internal docs directly.

## Why This Scope Is Correct

`jig` is not a broad platform product that needs a large documentation IA today. It is:

- a CLI
- with a small DSL
- used heavily through agents
- best understood through concrete examples

The highest-leverage public materials are:

- a strong value proposition
- a crisp quick start
- a handful of convincing examples
- strong agent integrations

That is enough to explain the tool without creating a second documentation burden.

## Site Scope

The site should stay a single page for now.

Recommended sections:

1. Hero
2. Why `jig` exists
3. How it works
4. Real examples
5. Agent integrations
6. Quick start
7. Reference links

This should be optimized for:

- introducing the tool quickly
- explaining when to use it
- showing deterministic before/after value
- sending users into the repo for deeper examples

## Information Architecture

### Public Site

The public site should answer:

1. What is `jig`?
2. Why is it useful for agentic coding?
3. What kinds of tasks is it good at?
4. How do I install and run it?
5. Where are the best examples?
6. How do I use it with Claude and Codex?

### Repo

The repo should remain the source of truth for:

- examples
- release/install details
- product README
- internal engineering docs
- eval evidence

### Internal Docs

The current [`docs/`](/Users/tylerobriant/code/tetra/jig/docs) directory is internal and product-facing. It includes architecture, invariants, release process, and requirements work. It should not be repurposed as a public docs site.

## Recommended Repo Layout

```text
jig/
  docs/                  # internal docs only
  eval/
  examples/              # public teaching surface
  site/                  # public landing page
  src/
```

## `examples/` Strategy

Examples should be first-class and do most of the teaching work.

Recommended structure:

```text
examples/
  add-service-test/
    recipe.yaml
    vars.json
    before/
    after/
    templates/
    README.md
  add-structured-logging/
    ...
  add-django-field/
    ...
  patch-view-contract/
    ...
```

Each example should include:

1. the problem being solved
2. the exact `jig` command
3. the recipe
4. the variables file
5. before state
6. after state
7. expected diff or explanation of what changed

This is more valuable than a large prose docs site because `jig` is fundamentally about repeatable code-shaping patterns.

## Landing Page Structure

### 1. Hero

Goal:
- explain the product in one pass

Include:
- short statement of what `jig` is
- one sentence on deterministic code generation for agents
- primary CTA to install or view repo
- secondary CTA to examples

### 2. Why It Exists

Goal:
- explain the problem boundary

Include:
- LLMs are good at reasoning and bad at repeating the same structural edits consistently
- `jig` separates reasoning from deterministic mutation
- mention tokens, cost, latency, and correctness drift

### 3. How It Works

Goal:
- show the model in one visual block

Include:
- recipe + vars -> deterministic file operations
- create / inject / replace / patch
- skill-local ownership of recipes

This should stay short.

### 4. Real Examples

Goal:
- prove the tool is concrete

Include:
- 3-5 example cards or sections
- each example should link into `examples/`
- each example should show:
  - task type
  - before/after summary
  - why deterministic output matters

Recommended initial examples:

1. add service test
2. add structured logging
3. add field across model/service/schema
4. patch a request/response contract

### 5. Agent Integrations

Goal:
- make the product operational for the real user workflow

Include:
- Claude integration link
- Codex integration link
- statement that `jig` is most useful when the agent has a strong skill that knows when to use it
- short guidance on using `jig` for routine, shape-constrained work

This section matters more than a large docs tree.

### 6. Quick Start

Goal:
- let someone copy one working command

Include:
- install command
- minimal `recipe.yaml`
- one `jig run ... --vars ...` example
- one sentence explaining expected output

This is the only docs-like section that absolutely needs to live on the page.

### 7. Reference Links

Goal:
- route deeper users to the right place

Include links to:
- GitHub repo
- `README.md`
- `examples/`
- release/install instructions
- Claude integration
- Codex integration

## What We Do Not Need Yet

We do not need:

- a separate public docs portal
- a docs theme
- a multi-page public IA
- direct rendering of internal docs
- a public mirror of architecture or requirements documents

Those add maintenance cost without helping the first release much.

## Design Direction

The landing page can be highly custom and animated.

That does not conflict with the one-page approach. In fact, it is a reason to keep the site small:

- one polished page is manageable
- a large docs site would pull the design toward utility layouts
- examples in the repo can handle most of the teaching burden

So the right split is:

- custom visual landing page on the subdomain
- concrete examples in the repo
- minimal quick start embedded on-page

## Framework Guidance

Two acceptable options:

1. plain static files in `site/`
2. a static Astro site in `site/`

Current recommendation:

- if the site is likely to remain one page for a while, plain static files are fine
- if public docs are likely to appear soon, Astro is a reasonable future-proofing choice

Either way, the content model should stay the same:

- one landing page
- no internal docs leakage
- examples in the repo

Because the current plan is explicitly one page plus links, static files are the simplest default.

## Public Messaging Constraints

The site should make these boundaries clear:

- `jig` is for deterministic, patterned code generation and patching
- it is especially useful for routine, shape-constrained backend work
- it complements agent reasoning rather than replacing it

The site should not overclaim:

- not "works for every coding task"
- not "replaces careful reasoning"
- not "needs a giant framework around it"

## Next Build Steps

1. add `site/` as a self-contained public site directory
2. keep it as a single-page landing page
3. add an `examples/` directory at repo root
4. create 3-5 strong examples with before/after states
5. add Claude and Codex integration links on the page
6. keep internal docs untouched

## Acceptance Criteria

- The repo SHALL keep public marketing content separate from internal engineering docs.
- The public site SHALL be shippable as a single page.
- The landing page SHALL include a quick-start section with install and one working `jig run` example.
- The repo SHALL expose an `examples/` directory as the primary deeper-learning surface.
- The public site SHALL link to agent integrations for Claude and Codex.
- The public site SHALL not depend on rendering files from the internal [`docs/`](/Users/tylerobriant/code/tetra/jig/docs) directory.
- The initial site architecture SHALL remain compatible with adding more public content later without requiring changes to `src/`, `eval/`, or internal docs.
