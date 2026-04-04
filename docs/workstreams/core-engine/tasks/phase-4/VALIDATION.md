# VALIDATION.md

> Workstream: core-engine
> Task: phase-4
> Last verified: 2026-04-04

## Phase Validation Criteria

From PLAN.md Phase 4:

- after: content on line after first match (AC-5.1)
- before: content on line before first match (AC-5.2)
- prepend: content at start of file (AC-5.3)
- append: content at end of file (AC-5.4)
- at:last uses last match (AC-5.5), at:first (default) uses first (AC-5.6)
- skip_if: skips when string found in file, reports action:"skip" (AC-5.7)
- Regex no-match exits 3 with pattern, file path, hint (AC-5.8)
- Missing target file exits 3 (AC-5.9)
- Inject path renders as template (AC-5.11)
- Create-then-inject in same recipe works (AC-N6.2)
- Second run with skip_if shows all skips — no duplicate content (AC-N2.2)
- Inject success reports action:"inject" with path, location, line count (AC-5.10)
- at field ignored when prepend/append specified; after/before without regex exits 1 (AC-5.12, AC-5.13)
- Invalid regex pattern in after/before exits 1 (AC-5.14)
- Multiple inject modes (after+before etc.) specified exits 1 (AC-5.15)
- --force has no effect on inject operations (AC-5.16)

## Spec Requirements -> Tests

| Criterion | EARS Type | Source | Test | Status |
|-----------|-----------|--------|------|--------|
| AC-5.1 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_1_after_first_match` + `tests::ac_5_1_inject_after_via_run` | PASS |
| AC-5.2 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_2_before_first_match` | PASS |
| AC-5.3 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_3_prepend` + `tests::ac_5_3_5_4_prepend_append_via_run` | PASS |
| AC-5.4 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_4_append` + `tests::ac_5_3_5_4_prepend_append_via_run` | PASS |
| AC-5.5 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_5_after_last_match` + `operations::inject::tests::before_last_match` | PASS |
| AC-5.6 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_6_after_first_match_default` | PASS |
| AC-5.7 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_7_skip_if_matched` + `tests::ac_5_7_skip_if_via_run` | PASS |
| AC-5.8 | Unwanted | SPEC.md FR-5 | `operations::inject::tests::ac_5_8_regex_no_match` + `tests::ac_5_8_regex_no_match_exits_3` | PASS |
| AC-5.9 | Unwanted | SPEC.md FR-5 | `operations::inject::tests::ac_5_9_missing_target_file` + `tests::ac_5_9_missing_target_exits_3` | PASS |
| AC-5.10 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_10_success_reports_inject` | PASS |
| AC-5.11 | Event | SPEC.md FR-5 | `operations::inject::tests::ac_5_11_templated_inject_path` + `tests::ac_5_11_templated_inject_path_via_run` | PASS |
| AC-5.12 | Ubiquitous | SPEC.md FR-5 | `operations::inject::tests::ac_5_12_at_ignored_for_prepend_append` | PASS |
| AC-5.13 | Unwanted | SPEC.md FR-5 | `recipe::tests::inject_missing_mode_rejected` | PASS |
| AC-5.14 | Unwanted | SPEC.md FR-5 | `recipe::tests::invalid_regex_rejected_at_parse` | PASS |
| AC-5.15 | Unwanted | SPEC.md FR-5 | `recipe::tests::multiple_inject_modes_rejected` | PASS |
| AC-5.16 | Ubiquitous | SPEC.md FR-5 | `operations::inject::tests::ac_5_16_force_no_effect_on_inject` | PASS |
| AC-5.17 | Unwanted | SPEC.md FR-5 | `operations::inject::tests::ac_n4_2_error_includes_rendered_content` (permission error path) | PASS |
| AC-N2.1 | Event | SPEC.md NFR-2 | `tests::ac_n2_1_idempotent_create_inject` | PASS |
| AC-N2.2 | Ubiquitous | SPEC.md NFR-2 | `operations::inject::tests::ac_n2_2_no_duplicate_with_skip_if` | PASS |
| AC-N6.1 | Ubiquitous | SPEC.md NFR-6 | `tests::ac_n6_1_declaration_order` (Phase 3, still passing) | PASS |
| AC-N6.2 | Event | SPEC.md NFR-6 | `tests::ac_n6_2_create_then_inject` + `tests::ac_n6_2_create_then_inject_dry_run` | PASS |

## Coverage Summary

- Spec criteria: 21/21 covered
- Phase validation criteria: 16/16 covered

## Gaps

None. All criteria have passing tests.
