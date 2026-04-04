# VALIDATION.md

> Workstream: core-engine
> Task: phase-4
> Last verified: 2026-04-03

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
| AC-5.1 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_1` | PENDING |
| AC-5.2 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_2` | PENDING |
| AC-5.3 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_3` | PENDING |
| AC-5.4 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_4` | PENDING |
| AC-5.5 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_5` | PENDING |
| AC-5.6 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_6` | PENDING |
| AC-5.7 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_7` | PENDING |
| AC-5.8 | Unwanted | SPEC.md FR-5 | `spec::fr-5::ac_5_8` | PENDING |
| AC-5.9 | Unwanted | SPEC.md FR-5 | `spec::fr-5::ac_5_9` | PENDING |
| AC-5.10 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_10` | PENDING |
| AC-5.11 | Event | SPEC.md FR-5 | `spec::fr-5::ac_5_11` | PENDING |
| AC-5.12 | Ubiquitous | SPEC.md FR-5 | `spec::fr-5::ac_5_12` | PENDING |
| AC-5.13 | Unwanted | SPEC.md FR-5 | `spec::fr-5::ac_5_13` | PENDING |
| AC-5.14 | Unwanted | SPEC.md FR-5 | `spec::fr-5::ac_5_14` | PENDING |
| AC-5.15 | Unwanted | SPEC.md FR-5 | `spec::fr-5::ac_5_15` | PENDING |
| AC-5.16 | Ubiquitous | SPEC.md FR-5 | `spec::fr-5::ac_5_16` | PENDING |
| AC-5.17 | Unwanted | SPEC.md FR-5 | `spec::fr-5::ac_5_17` | PENDING |
| AC-N2.1 | Event | SPEC.md NFR-2 | `spec::nfr-2::ac_n2_1` | PENDING |
| AC-N2.2 | Ubiquitous | SPEC.md NFR-2 | `spec::nfr-2::ac_n2_2` | PENDING |
| AC-N6.1 | Ubiquitous | SPEC.md NFR-6 | `spec::nfr-6::ac_n6_1` | PENDING |
| AC-N6.2 | Event | SPEC.md NFR-6 | `spec::nfr-6::ac_n6_2` | PENDING |

## Coverage Summary

- Spec criteria: 0/21 covered
- Phase validation criteria: 0/23 covered

## Gaps

All criteria need test implementations.
