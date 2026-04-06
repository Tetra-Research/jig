# Eval Schema Compatibility Plan (Finding #2)

Date: 2026-04-06  
Status: Implemented (2026-04-06)

## Objective

Fix metric trustworthiness issues caused by mixed result schemas while preserving our ability to inspect both legacy and current experiment archives.

## Problem Statement

Current eval reporting can silently bias efficiency metrics downward because missing token/cost fields are treated as `0` in some analysis paths.

Known example: `eval/results/archive/results-mixed-schema-20260406T114302.jsonl`
- Rows: 17
- Rows with full efficiency fields: 5
- Remaining rows are legacy schema without full token breakdown

Mixing these rows currently risks misleading averages.

## Non-Negotiables

1. Legacy files must remain readable.
2. New files must remain fully analyzable.
3. Mixed-schema files must never produce silently skewed efficiency metrics.
4. Readiness/CI paths must have a strict mode that fails on untrustworthy input.

## Execution Plan

1. Add schema diagnostics at ingestion.
   - Extend `readResults` to classify rows (`v1_legacy`, `v2_current`, `invalid`) and return diagnostics (counts + line warnings).
   - Keep raw valid rows available for score metrics.

2. Separate score metrics from efficiency metrics.
   - Score metrics (assertions, totals, baseline delta) use all valid rows.
   - Efficiency metrics (input/output/total tokens, cost) use only rows with complete efficiency fields.

3. Remove zero fallbacks for missing efficiency fields.
   - Replace `?? 0` behavior in reporting/analysis with:
     - `N/A` when coverage is incomplete, or
     - means computed on covered subset plus explicit coverage display.

4. Add schema policy mode.
   - `strict` mode (default for readiness/CI): fail on mixed schemas or malformed lines.
   - `compat` mode (default for exploratory analysis): allow mixed files with explicit warnings and coverage stats.

5. Add archive hygiene utility.
   - Add command/script to split mixed JSONL into schema-homogeneous outputs for historical comparisons (`v1` and `v2`).

6. Add tests for regressions.
   - Malformed JSONL handling is surfaced, not silent.
   - Mixed schema detection behaves correctly in strict/compat modes.
   - Efficiency aggregates never use implicit zero substitution.
   - Coverage reporting is present and accurate.

7. Update docs.
   - Document strict vs compat usage and examples for legacy/new/mixed archives.
   - Document expected operator behavior before publishing findings.

## Proposed File Touches

- `eval/harness/results.ts`
- `eval/harness/report.ts`
- `eval/experiments/analyze-gradient.ts`
- `eval/harness/run.ts`
- `eval/harness/test.ts`
- `eval/experiments/README.md`
- Optional new utility script in `eval/experiments/` or `eval/harness/`

## Acceptance Criteria

1. Given a mixed-schema JSONL, strict mode exits non-zero with clear diagnostics.
2. Given a mixed-schema JSONL, compat mode succeeds and shows efficiency coverage (for example `5/17`) with no silent zero substitution.
3. Given legacy-only JSONL, score reporting works and efficiency is explicitly marked unavailable/partial.
4. Given current-schema JSONL, all existing efficiency metrics still compute and report correctly.
5. Eval harness tests include new mixed/legacy fixtures and pass.

## Rollout

1. Commit A: ingestion diagnostics + schema policy + tests.
2. Commit B: reporting/analysis updates + docs + optional splitter utility.

This keeps behavior changes easy to review and rollback if needed.
