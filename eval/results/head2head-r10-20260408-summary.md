# Head-to-Head r10 Summary (2026-04-08)

- Run file: `eval/results/head2head-results-h2h-r10-20260408.jsonl`
- Pairs file: `eval/results/head2head-pairs-h2h-r10-20260408.jsonl`
- Prompt source: `directed`
- Thinking mode: enabled
- Reps: 10 per scenario/arm

## Per-Skill Metrics

| Scenario | Arm | Pass | Avg Score | Avg Duration ms | Avg Output Tokens | Avg Context Tokens | Avg Total Tokens | Avg Tool Calls | Avg Cost USD | Jig Used | Thinking Captured |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| h2h-deterministic-service-test | control | 0.0% | 0.00 | 21,127 | 1,247 | 86,561 | 87,808 | 3.00 | 0.3381 | 0.0% | 100.0% |
| h2h-deterministic-service-test | jig | 100.0% | 1.00 | 18,277 | 782 | 105,623 | 106,405 | 4.00 | 0.3410 | 100.0% | 100.0% |
| h2h-query-layer-discipline | control | 0.0% | 0.00 | 18,993 | 1,049 | 78,172 | 79,221 | 2.60 | 0.3101 | 0.0% | 100.0% |
| h2h-query-layer-discipline | jig | 100.0% | 1.00 | 16,225 | 866 | 111,273 | 112,139 | 4.10 | 0.3690 | 100.0% | 100.0% |
| h2h-schema-migration-safety | control | 100.0% | 1.00 | 48,192 | 3,409 | 371,956 | 375,365 | 14.60 | 1.0064 | 0.0% | 100.0% |
| h2h-schema-migration-safety | jig | 100.0% | 1.00 | 15,978 | 966 | 100,738 | 101,704 | 3.50 | 0.3614 | 100.0% | 100.0% |
| h2h-structured-logging-contract | control | 0.0% | 0.72 | 19,564 | 1,232 | 131,014 | 132,247 | 5.10 | 0.4277 | 0.0% | 100.0% |
| h2h-structured-logging-contract | jig | 100.0% | 1.00 | 18,514 | 1,114 | 116,221 | 117,335 | 5.90 | 0.4053 | 100.0% | 100.0% |
| h2h-view-contract-enforcer | control | 0.0% | 0.23 | 36,223 | 2,440 | 217,266 | 219,706 | 8.00 | 0.6702 | 0.0% | 100.0% |
| h2h-view-contract-enforcer | jig | 100.0% | 1.00 | 15,412 | 888 | 98,016 | 98,903 | 3.40 | 0.3527 | 100.0% | 100.0% |

## Notes

- In multiple control trials, the model interpreted checklist skills as analysis-only and returned guidance without file edits.
- Example artifact: `eval/results/head2head-artifacts/h2h-r10-20260408/2026-04-08T22-50-04-529Z__h2h-deterministic-service-test__claude-code__control__rep1__927opf/combined.log` shows "No files created".
- This means current control-vs-jig numbers include a prompt/skill-mode confound, not only implementation-quality differences.
