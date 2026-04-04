# VALIDATION.md

> Workstream: core-engine
> Task: phase-2
> Last verified: 2026-04-03

## Phase Validation Criteria

From PLAN.md Phase 2:

- All 13 filters produce correct output per AC-3.4 through AC-3.14
- `jig render template.j2 --vars '{"class_name": "BookingService"}'` renders to stdout
- Type mismatch exits 4 with expected vs actual type (AC-2.7)
- Missing required variable exits 4 with variable name and hint (AC-2.5)
- Merge precedence: inline --vars wins over --vars-stdin wins over --vars-file wins over defaults (AC-2.4)
- Undefined template variable exits 2 with "did you mean?" hint (AC-3.17)
- Template syntax error exits 2 with file path and line number (AC-3.18)
- Same inputs produce byte-identical output across runs (AC-N1.1)
- `jig render template.j2 --vars '...' --to output.txt` writes to file instead of stdout (AC-7.4)
- Multiple validation errors accumulated and reported together (AC-2.11)

## Spec Requirements -> Tests

| Criterion | EARS Type | Source | Test | Status |
|-----------|-----------|--------|------|--------|
| AC-2.1 | Event | SPEC.md FR-2 | `variables::tests::ac_2_1_parse_inline_vars` | PASS |
| AC-2.2 | Event | SPEC.md FR-2 | `variables::tests::ac_2_2_parse_vars_file` | PASS |
| AC-2.3 | Event | SPEC.md FR-2 | `cli::ac_2_3_vars_stdin` | PASS |
| AC-2.4 | Event | SPEC.md FR-2 | `variables::tests::ac_2_4_merge_precedence`, `variables::tests::ac_2_4_defaults_lowest_precedence` | PASS |
| AC-2.5 | Event | SPEC.md FR-2 | `variables::tests::ac_2_5_required_missing` | PASS |
| AC-2.6 | Event | SPEC.md FR-2 | `variables::tests::ac_2_6_default_fallback` | PASS |
| AC-2.7 | Unwanted | SPEC.md FR-2 | `variables::tests::ac_2_7_type_mismatch_*` (4 tests) | PASS |
| AC-2.8 | Event | SPEC.md FR-2 | `variables::tests::ac_2_8_enum_valid`, `variables::tests::ac_2_8_enum_rejection` | PASS |
| AC-2.9 | Event | SPEC.md FR-2 | `variables::tests::ac_2_9_array_items_valid`, `variables::tests::ac_2_9_array_item_type_mismatch` | PASS |
| AC-2.10 | Ubiquitous | SPEC.md FR-2 | `variables::tests::ac_2_10_all_six_types` | PASS |
| AC-2.11 | Ubiquitous | SPEC.md FR-2 | `variables::tests::ac_2_11_multiple_errors_accumulated` | PASS |
| AC-2.12 | Ubiquitous | SPEC.md FR-2 | `variables::tests::ac_2_12_extra_keys_pass_through` | PASS |
| AC-2.13 | Unwanted | SPEC.md FR-2 | `variables::tests::ac_2_13_invalid_json_inline` | PASS |
| AC-2.14 | Unwanted | SPEC.md FR-2 | `variables::tests::ac_2_14_vars_file_not_found` | PASS |
| AC-2.15 | Unwanted | SPEC.md FR-2 | `variables::tests::ac_2_15_vars_file_invalid_json` | PASS |
| AC-2.16 | Event | SPEC.md FR-2 | `variables::tests::ac_2_16_no_sources_uses_defaults` | PASS |
| AC-3.1 | Event | SPEC.md FR-3 | `renderer::tests::ac_3_1_variable_substitution` | PASS |
| AC-3.2 | Event | SPEC.md FR-3 | `renderer::tests::ac_3_2_conditionals` | PASS |
| AC-3.3 | Event | SPEC.md FR-3 | `renderer::tests::ac_3_3_for_loops` | PASS |
| AC-3.4 | Ubiquitous | SPEC.md FR-3 | `filters::tests::ac_3_4_all_filters_registered` | PASS |
| AC-3.5 | Event | SPEC.md FR-3 | `filters::tests::ac_3_5_snakecase` | PASS |
| AC-3.6 | Event | SPEC.md FR-3 | `filters::tests::ac_3_6_camelcase` | PASS |
| AC-3.7 | Event | SPEC.md FR-3 | `filters::tests::ac_3_7_pascalcase` | PASS |
| AC-3.8 | Event | SPEC.md FR-3 | `filters::tests::ac_3_8_kebabcase` | PASS |
| AC-3.9 | Event | SPEC.md FR-3 | `filters::tests::ac_3_9_replace` | PASS |
| AC-3.10 | Event | SPEC.md FR-3 | `filters::tests::ac_3_10_pluralize` | PASS |
| AC-3.11 | Event | SPEC.md FR-3 | `filters::tests::ac_3_11_singularize` | PASS |
| AC-3.12 | Event | SPEC.md FR-3 | `filters::tests::ac_3_12_quote` | PASS |
| AC-3.13 | Event | SPEC.md FR-3 | `filters::tests::ac_3_13_indent_all_lines`, `filters::tests::ac_3_13_indent_skip_first` | PASS |
| AC-3.14 | Event | SPEC.md FR-3 | `filters::tests::ac_3_14_join` | PASS |
| AC-3.15 | Event | SPEC.md FR-3 | `renderer::tests::ac_3_15_comments_stripped` | PASS |
| AC-3.16 | Event | SPEC.md FR-3 | `renderer::tests::ac_3_16_raw_blocks` | PASS |
| AC-3.17 | Unwanted | SPEC.md FR-3 | `renderer::tests::ac_3_17_undefined_variable_did_you_mean` | PASS |
| AC-3.18 | Unwanted | SPEC.md FR-3 | `renderer::tests::ac_3_18_syntax_error` | PASS |
| AC-7.1 | Event | SPEC.md FR-7 | `tests::ac_7_1_validate_command_valid`, `tests::ac_7_1_validate_json_output` | PASS |
| AC-7.2 | Event | SPEC.md FR-7 | `tests::ac_7_2_vars_command` | PASS |
| AC-7.3 | Event | SPEC.md FR-7 | `tests::ac_7_3_render_to_stdout`, `tests::ac_7_3_render_with_vars_file` | PASS |
| AC-7.4 | Event | SPEC.md FR-7 | `tests::ac_7_4_render_to_file`, `tests::ac_7_4_render_to_creates_dirs` | PASS |
| AC-7.6 | Ubiquitous | SPEC.md FR-7 | `tests::ac_7_6_json_flag_exists`, `tests::ac_7_6_var_options_exist` | PASS |
| AC-7.7 | Event | SPEC.md FR-7 | `tests::ac_7_7_version_configured` | PASS |
| AC-N1.1 | Ubiquitous | SPEC.md NFR-1 | `renderer::tests::ac_n1_1_deterministic` | PASS |
| AC-N1.2 | Ubiquitous | SPEC.md NFR-1 | `renderer::tests::ac_n1_2_no_nondeterminism` | PASS |

Note: AC-7.5 (`jig run`) is deferred to Phase 3 (create + inject operations required).

## Coverage Summary

- Spec criteria: 42/42 covered (AC-7.5 deferred to Phase 3)
- Phase validation criteria: 10/10 covered

## Gaps

None. All Phase 2 acceptance criteria have passing tests.
