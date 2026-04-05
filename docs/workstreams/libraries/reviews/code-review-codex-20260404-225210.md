# Codex Output (extracted from raw JSON)

Reviewing the `libraries` workstream now with a spec-first adversarial pass. I’ll first locate `SPEC.md`, `PLAN.md`, and the implementation/tests tied to this workstream, then map findings to EARS criteria and invariants.

I found the `libraries` implementation and workstream artifacts. Next I’m reading `docs/workstreams/libraries/SPEC.md` and the `src/library/*` + CLI wiring to check each acceptance criterion and invariant against actual behavior.

The `libraries` folder doesn’t currently contain top-level `SPEC.md`/`PLAN.md`; I’m now pulling the synthesized/spec-population artifacts in `docs/workstreams/libraries/exec/` to use as the effective spec baseline for this review.

I confirmed there’s no committed `libraries/SPEC.md` or `PLAN.md`; the synthesized file is just a planning failure log. I’m now extracting the effective v0.4 library requirements from `jig.md`, `ROADMAP.md`, and existing `libraries` code/tests so I can still do a spec-alignment review and explicitly call out missing formal AC traceability.

I’ve now read the library modules and CLI handlers. Next I’m mapping them against the v0.4 requirements in `jig.md` (manifest format, install locations, add/remove/update/list, recipe/workflow discovery, conventions overrides) and checking tests for coverage gaps.

I’ve mapped the spec sections and identified likely mismatch areas (URL install, update semantics, conventions integration, overrides/extensions, and library-qualified execution). I’ll run the library-related tests now to confirm what behavior is actually enforced versus missing.

**Verdict:** Request Changes

**Critical** (must fix before merge):
- No canonical workstream spec exists to review against: `SPEC.md` is required/canonical by process, but `libraries` still has none, so EARS AC traceability is impossible ([AGENTS.md#L23](/Users/tylerobriant/code/tetra/jig/AGENTS.md#L23), [docs/ROADMAP.md#L75](/Users/tylerobriant/code/tetra/jig/docs/ROADMAP.md#L75)).
- `library add` is vulnerable to path traversal via manifest `name` (`../../...` or absolute paths) because only non-empty is validated, then directly joined into install path ([src/library/manifest.rs#L88](/Users/tylerobriant/code/tetra/jig/src/library/manifest.rs#L88), [src/library/install.rs#L81](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L81)).
- `library update` can corrupt library identity: it does not verify source manifest name matches the target `<name>`, leading to list/remove inconsistencies ([src/library/install.rs#L162](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L162), [src/library/install.rs#L184](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L184), [src/library/install.rs#L220](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L220), [src/library/install.rs#L295](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L295)).
- `library info` silently returns success with empty metadata when `recipe.yaml` is invalid/missing parseable content, violating graceful-fail expectations for LLM callers ([src/library/discover.rs#L82](/Users/tylerobriant/code/tetra/jig/src/library/discover.rs#L82), [src/library/discover.rs#L101](/Users/tylerobriant/code/tetra/jig/src/library/discover.rs#L101)).

**Major** (should fix):
- Spec mismatch: `jig library add <url|path>` and `jig library update <name>` are specified, but implementation is path-only add plus `update <name> <path>` ([jig.md#L1258](/Users/tylerobriant/code/tetra/jig/jig.md#L1258), [jig.md#L1267](/Users/tylerobriant/code/tetra/jig/jig.md#L1267), [src/main.rs#L103](/Users/tylerobriant/code/tetra/jig/src/main.rs#L103), [src/main.rs#L116](/Users/tylerobriant/code/tetra/jig/src/main.rs#L116), [src/main.rs#L681](/Users/tylerobriant/code/tetra/jig/src/main.rs#L681)).
- Library-qualified execution is not integrated (`jig run django/...`, `jig workflow django/...` examples fail because handlers treat input as filesystem paths) ([jig.md#L1229](/Users/tylerobriant/code/tetra/jig/jig.md#L1229), [jig.md#L1487](/Users/tylerobriant/code/tetra/jig/jig.md#L1487), [src/main.rs#L323](/Users/tylerobriant/code/tetra/jig/src/main.rs#L323), [src/main.rs#L562](/Users/tylerobriant/code/tetra/jig/src/main.rs#L562), [src/library/discover.rs#L146](/Users/tylerobriant/code/tetra/jig/src/library/discover.rs#L146)).
- Convention overrides are implemented as a module but not wired into execution paths, so v0.4 convention behavior is not delivered end-to-end ([docs/ROADMAP.md#L102](/Users/tylerobriant/code/tetra/jig/docs/ROADMAP.md#L102), [src/library/conventions.rs#L83](/Users/tylerobriant/code/tetra/jig/src/library/conventions.rs#L83), [src/main.rs#L323](/Users/tylerobriant/code/tetra/jig/src/main.rs#L323)).
- Required extension/override features are missing (`.jig/overrides/...`, `.jig/extensions/...`) ([docs/ROADMAP.md#L107](/Users/tylerobriant/code/tetra/jig/docs/ROADMAP.md#L107), [docs/ARCHITECTURE.md#L583](/Users/tylerobriant/code/tetra/jig/docs/ARCHITECTURE.md#L583), [src/library/mod.rs#L1](/Users/tylerobriant/code/tetra/jig/src/library/mod.rs#L1)).
- `library list` silently drops malformed installed libraries instead of surfacing actionable errors ([src/library/install.rs#L284](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L284)).
- Test coverage is strong for implemented local-path lifecycle, but missing for spec-critical behaviors above (URL add, name-safe update, library-qualified run/workflow, overrides/extensions) ([tests/library.rs#L80](/Users/tylerobriant/code/tetra/jig/tests/library.rs#L80)).

**Minor** (nice to fix):
- Listing order is based on raw `read_dir` iteration and not sorted, so output ordering may be non-deterministic across environments ([src/library/install.rs#L255](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L255), [src/library/install.rs#L264](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L264)).

**Strengths** (what's done well):
- Structured error shape is consistent and generally high quality (`what/where/why/hint`) across library operations ([src/library/install.rs#L65](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L65)).
- Project-local precedence over global is implemented clearly for find/list paths ([src/library/install.rs#L200](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L200), [src/library/install.rs#L218](/Users/tylerobriant/code/tetra/jig/src/library/install.rs#L218)).
- Good CLI integration coverage for the currently implemented subcommands and lifecycle flow ([tests/library.rs#L460](/Users/tylerobriant/code/tetra/jig/tests/library.rs#L460)).
