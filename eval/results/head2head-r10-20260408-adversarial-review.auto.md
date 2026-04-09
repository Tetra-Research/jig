# Head-to-Head Adversarial Review

- Results: `results/head2head-results-h2h-r10-20260408.jsonl`
- Rows analyzed: 100
- Scenarios analyzed: 5

## Scenario Summary

| Scenario | No-op File Score | Control Pass | Control Edit Rate | Control Analysis-Only | Jig Pass |
| --- | ---: | ---: | ---: | ---: | ---: |
| h2h-deterministic-service-test | 0.0% | 0.0% | 0.0% | 100.0% | 100.0% |
| h2h-query-layer-discipline | 31.1% | 0.0% | 0.0% | 100.0% | 100.0% |
| h2h-schema-migration-safety | 25.0% | 100.0% | 100.0% | 0.0% | 100.0% |
| h2h-structured-logging-contract | 38.9% | 0.0% | 100.0% | 0.0% | 100.0% |
| h2h-view-contract-enforcer | 48.4% | 0.0% | 30.0% | 70.0% | 100.0% |

## Key Findings

- `h2h-deterministic-service-test`: control often behaved as analysis-only (10/10) instead of editing files.
- `h2h-query-layer-discipline`: control often behaved as analysis-only (10/10) instead of editing files.
- `h2h-query-layer-discipline`: no-op baseline already scores 31.1% on `file_score`, so raw file similarity overstates progress on untouched runs.
- `h2h-schema-migration-safety`: no-op baseline already scores 25.0% on `file_score`, so raw file similarity overstates progress on untouched runs.
- `h2h-schema-migration-safety`: control reaches full assertion score but still diverges materially from expected file shape (mean file_score 82.1%).
- `h2h-structured-logging-contract`: no-op baseline already scores 38.9% on `file_score`, so raw file similarity overstates progress on untouched runs.
- `h2h-structured-logging-contract`: control does edit in some or all runs, but its outputs are less consistent than jig-backed runs.
- `h2h-view-contract-enforcer`: control often behaved as analysis-only (7/10) instead of editing files.
- `h2h-view-contract-enforcer`: no-op baseline already scores 48.4% on `file_score`, so raw file similarity overstates progress on untouched runs.
- `h2h-view-contract-enforcer`: control does edit in some or all runs, but its outputs are less consistent than jig-backed runs.

## Comparison Examples

### h2h-view-contract-enforcer rep 10

- Control: score=0.78, file_score=0.35
- Jig: score=1.00, file_score=1.00
- Control failed assertions: request_contract.is_valid(raise_exception=True)
- Control result: `Checklist applied: 1. Input validated via 'EntitySummaryRequest' before business logic (views.py:18-19). 2. Explicit auth: 'IsAuthenticated' + 'has_perm' check at entrypoint (vi...`
- Jig result: `Recipe applied successfully. Files updated: 'views.py', 'schemas.py', 'urls.py', 'tests/test_views.py'.`
- Control artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-27-33-855Z__h2h-view-contract-enforcer__claude-code__control__rep10__1e877a`
- Jig artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-27-53-813Z__h2h-view-contract-enforcer__claude-code__jig__rep10__yd01at`

### h2h-view-contract-enforcer rep 2

- Control: score=0.78, file_score=0.38
- Jig: score=1.00, file_score=1.00
- Control failed assertions: request_contract.is_valid(raise_exception=True)
- Control result: `All checklist items from view-contract-enforcer are satisfied: request validation via 'EntitySummaryRequest', explicit 'IsAuthenticated' permission, service handoff to 'build_en...`
- Jig result: `Jig recipe executed successfully. Changes applied to views.py, schemas.py, urls.py, and tests/test_views.py.`
- Control artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-21-03-856Z__h2h-view-contract-enforcer__claude-code__control__rep2__e5e8jo`
- Jig artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-21-20-805Z__h2h-view-contract-enforcer__claude-code__jig__rep2__gzojtl`

### h2h-view-contract-enforcer rep 7

- Control: score=0.78, file_score=0.46
- Jig: score=1.00, file_score=1.00
- Control failed assertions: request_contract.is_valid(raise_exception=True)
- Control result: `Checklist complete: 1. **Input validation**: 'EntitySummaryRequest' validates at the view boundary via 'is_valid(raise_exception=True)' before any business logic. 2. **Authoriza...`
- Jig result: `Jig applied the view contract successfully across 'views.py', 'schemas.py', 'urls.py', and 'tests/test_views.py'.`
- Control artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-25-03-366Z__h2h-view-contract-enforcer__claude-code__control__rep7__k7e26d`
- Jig artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-25-17-328Z__h2h-view-contract-enforcer__claude-code__jig__rep7__rk7out`

### h2h-structured-logging-contract rep 1

- Control: score=0.72, file_score=0.76
- Jig: score=1.00, file_score=1.00
- Control failed assertions: core_service.create_record.done
- Control result: `Applied the structured-logging-contract to 'services/core_service.py:create_record': - Module logger added - Start event: 'core_service.create_record.start' at entry - Complete ...`
- Jig result: `Recipe applied successfully to 'services/core_service.py': logger setup injected, plus start/done log patches around 'create_record'.`
- Control artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-13-14-217Z__h2h-structured-logging-contract__claude-code__control__rep1__798rx5`
- Jig artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-13-31-800Z__h2h-structured-logging-contract__claude-code__jig__rep1__24r8im`

### h2h-structured-logging-contract rep 2

- Control: score=0.72, file_score=0.76
- Jig: score=1.00, file_score=1.00
- Control failed assertions: core_service.create_record.done
- Control result: `Applied the structured logging contract to 'services/core_service.py:7': - Added module logger - Start event 'core_service.create_record.start' at entry - Completion event 'core...`
- Jig result: `Skill applied successfully. Logger setup injected, and start/done log statements patched into 'create_record' in services/core_service.py:7.`
- Control artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-13-50-422Z__h2h-structured-logging-contract__claude-code__control__rep2__5902ou`
- Jig artifacts: `results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-14-09-278Z__h2h-structured-logging-contract__claude-code__jig__rep2__7a4m2l`
