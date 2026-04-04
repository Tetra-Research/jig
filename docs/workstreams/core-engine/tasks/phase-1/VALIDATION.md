# VALIDATION.md

> Workstream: core-engine
> Task: phase-1
> Last verified: 2026-04-03

## Phase Validation Criteria

From PLAN.md Phase 1:

- `jig validate recipe.yaml` parses the example recipe from jig.md and exits 0
- `jig validate bad.yaml` exits 1 with a clear error naming the problem
- `jig vars recipe.yaml` outputs JSON matching the SPEC schema (type, required, default, description)
- Template paths resolve relative to recipe file, not cwd
- AC-1.1 through AC-1.15 have corresponding unit tests (AC-1.11 empty files array tested in Phase 3 when `jig run` exists)
- `jig validate` output includes variable names and operation types
- AC-N5.1 exit codes are correct for recipe validation errors

## Spec Requirements -> Tests

| Criterion | EARS Type | Source | Test | Status |
|-----------|-----------|--------|------|--------|
| AC-1.1 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_1_valid_recipe_parses` | PASS |
| AC-1.2 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_2_variable_fields_parse` | PASS |
| AC-1.3 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_3_create_op_parses` | PASS |
| AC-1.4 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_4_inject_op_parses` | PASS |
| AC-1.5 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_5_malformed_yaml` | PASS |
| AC-1.6 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_6_missing_template_field` | PASS |
| AC-1.7 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_7_template_relative_to_recipe` | PASS |
| AC-1.8 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_8_missing_template_file` | PASS |
| AC-1.9 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_9_optional_metadata` | PASS |
| AC-1.10 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_10_unknown_op_replace`, `recipe::tests::ac_1_10_unknown_op_patch` | PASS |
| AC-1.11 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_11_empty_files_array` (parse-level; run behavior in Phase 3) | PASS |
| AC-1.12 | Event | SPEC.md FR-1 | `recipe::tests::ac_1_12_no_variables` | PASS |
| AC-1.13 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_13_recipe_file_not_found` | PASS |
| AC-1.14 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_14_ambiguous_op_type` | PASS |
| AC-1.15 | Unwanted | SPEC.md FR-1 | `recipe::tests::ac_1_15_missing_op_type` | PASS |
| AC-7.1 | Event | SPEC.md FR-7 | `tests::ac_7_1_validate_command_valid`, `tests::ac_7_1_validate_json_output`, `tests::ac_7_1_validate_command_invalid`, `tests::ac_7_1_validate_json_lists_vars_and_ops` | PASS |
| AC-7.2 | Event | SPEC.md FR-7 | `tests::ac_7_2_vars_command` | PASS |
| AC-7.3 | Event | SPEC.md FR-7 | Phase 2 (render command) | DEFERRED |
| AC-7.4 | Event | SPEC.md FR-7 | Phase 2 (render --to) | DEFERRED |
| AC-7.5 | Event | SPEC.md FR-7 | Phase 3 (run command) | DEFERRED |
| AC-7.6 | Ubiquitous | SPEC.md FR-7 | `tests::ac_7_6_json_flag_exists` (partial; remaining flags in Phase 2-4) | PASS |
| AC-7.7 | Event | SPEC.md FR-7 | `tests::ac_7_7_version_configured` | PASS |
| AC-N4.1 | Ubiquitous | SPEC.md NFR-4 | `recipe::tests::ac_n4_1_errors_have_all_fields` | PASS |
| AC-N4.2 | Event | SPEC.md NFR-4 | Phase 3 (file operation errors) | DEFERRED |
| AC-N4.3 | Event | SPEC.md NFR-4 | Phase 2 (template rendering errors) | DEFERRED |
| AC-N4.4 | Event | SPEC.md NFR-4 | Phase 2 (variable validation errors) | DEFERRED |
| AC-N5.1 | Ubiquitous | SPEC.md NFR-5 | `recipe::tests::ac_n5_1_exit_code_is_1`, `error::tests::exit_code_mapping` | PASS |
| AC-N5.2 | Ubiquitous | SPEC.md NFR-5 | `tests::ac_n5_2_recipe_validation_first` (partial; full pipeline ordering in Phase 3+) | PASS |

## Coverage Summary

- Spec criteria: 23/28 covered (5 deferred to later phases)
- Phase validation criteria: 7/7 covered
- Additional tests: `recipe::tests::invalid_regex_rejected_at_parse`, `recipe::tests::multiple_inject_modes_rejected`, `recipe::tests::inject_missing_mode_rejected`, `error::tests::structured_error_has_all_fields`, `error::tests::structured_error_serializes_where_field`, `variables::tests::vars_json_includes_all_fields`, `variables::tests::vars_json_preserves_declaration_order`

## Gaps

None for Phase 1 scope. Deferred criteria will be covered in their respective phases.
