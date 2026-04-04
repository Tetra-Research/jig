# VALIDATION.md

> Workstream: core-engine
> Task: phase-3
> Last verified: 2026-04-03

## Phase Validation Criteria

From PLAN.md Phase 3:

- `jig run recipe.yaml --vars '...'` creates files at templated paths (AC-4.1, AC-4.2)
- Parent directories created automatically (AC-4.3)
- skip_if_exists: true skips existing files with action:"skip" (AC-4.4)
- Default (skip_if_exists: false) errors on existing file without --force (AC-4.5)
- --force overwrites regardless (AC-4.6)
- --base-dir changes output root (AC-4.7)
- --dry-run produces output but writes nothing (AC-6.8)
- JSON output when piped, human output when TTY (AC-6.1, AC-6.2)
- --json forces JSON (AC-6.3), --quiet suppresses non-errors (AC-6.4)
- Operations execute in declaration order (AC-N6.1)
- Second run with skip_if_exists: true reports all skips (AC-N2.1)

## Spec Requirements -> Tests

| Criterion | EARS Type | Source | Test | Status |
|-----------|-----------|--------|------|--------|
| AC-4.1 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_1` | PENDING |
| AC-4.2 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_2` | PENDING |
| AC-4.3 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_3` | PENDING |
| AC-4.4 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_4` | PENDING |
| AC-4.5 | Unwanted | SPEC.md FR-4 | `spec::fr-4::ac_4_5` | PENDING |
| AC-4.6 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_6` | PENDING |
| AC-4.7 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_7` | PENDING |
| AC-4.8 | Event | SPEC.md FR-4 | `spec::fr-4::ac_4_8` | PENDING |
| AC-4.9 | Unwanted | SPEC.md FR-4 | `spec::fr-4::ac_4_9` | PENDING |
| AC-4.10 | Unwanted | SPEC.md FR-4 | `spec::fr-4::ac_4_10` | PENDING |
| AC-6.1 | State | SPEC.md FR-6 | `spec::fr-6::ac_6_1` | PENDING |
| AC-6.2 | State | SPEC.md FR-6 | `spec::fr-6::ac_6_2` | PENDING |
| AC-6.3 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_3` | PENDING |
| AC-6.4 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_4` | PENDING |
| AC-6.5 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_5` | PENDING |
| AC-6.6 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_6` | PENDING |
| AC-6.7 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_7` | PENDING |
| AC-6.8 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_8` | PENDING |
| AC-6.9 | Ubiquitous | SPEC.md FR-6 | `spec::fr-6::ac_6_9` | PENDING |
| AC-6.10 | Event | SPEC.md FR-6 | `spec::fr-6::ac_6_10` | PENDING |
| AC-6.11 | Ubiquitous | SPEC.md FR-6 | `spec::fr-6::ac_6_11` | PENDING |
| AC-7.1 | Event | SPEC.md FR-7 | `spec::fr-7::ac_7_1` | PENDING |
| AC-7.2 | Event | SPEC.md FR-7 | `spec::fr-7::ac_7_2` | PENDING |
| AC-7.3 | Event | SPEC.md FR-7 | `spec::fr-7::ac_7_3` | PENDING |
| AC-7.4 | Event | SPEC.md FR-7 | `spec::fr-7::ac_7_4` | PENDING |
| AC-7.5 | Event | SPEC.md FR-7 | `spec::fr-7::ac_7_5` | PENDING |
| AC-7.6 | Ubiquitous | SPEC.md FR-7 | `spec::fr-7::ac_7_6` | PENDING |
| AC-7.7 | Event | SPEC.md FR-7 | `spec::fr-7::ac_7_7` | PENDING |
| AC-N2.1 | Event | SPEC.md NFR-2 | `spec::nfr-2::ac_n2_1` | PENDING |
| AC-N2.2 | Ubiquitous | SPEC.md NFR-2 | `spec::nfr-2::ac_n2_2` | PENDING |
| AC-N4.1 | Ubiquitous | SPEC.md NFR-4 | `spec::nfr-4::ac_n4_1` | PENDING |
| AC-N4.2 | Event | SPEC.md NFR-4 | `spec::nfr-4::ac_n4_2` | PENDING |
| AC-N4.3 | Event | SPEC.md NFR-4 | `spec::nfr-4::ac_n4_3` | PENDING |
| AC-N4.4 | Event | SPEC.md NFR-4 | `spec::nfr-4::ac_n4_4` | PENDING |
| AC-N6.1 | Ubiquitous | SPEC.md NFR-6 | `spec::nfr-6::ac_n6_1` | PENDING |
| AC-N6.2 | Event | SPEC.md NFR-6 | `spec::nfr-6::ac_n6_2` | PENDING |

## Coverage Summary

- Spec criteria: 0/36 covered
- Phase validation criteria: 0/22 covered

## Gaps

All criteria need test implementations.
