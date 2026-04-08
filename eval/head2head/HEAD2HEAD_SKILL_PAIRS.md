# Head-to-Head Skill Pairs

Canonical pair map for control-vs-jig comparisons.

## Pair Set

1. `schema-migration-safety`
- Control: `profiles/control/skills/schema-migration-safety/SKILL.md`
- Jig: `profiles/jig/skills/schema-migration-safety/SKILL.md`
- Vars: `app_label`, `model_name`, `field_name`, `field_type`, `add_field_kwargs`, `final_field_kwargs`, `previous_migration`, `add_migration_name`, `finalize_migration_name`, `backfill_value`
- Focus: two-step migration safety and backfill discipline

2. `view-contract-enforcer`
- Control: `profiles/control/skills/view-contract-enforcer/SKILL.md`
- Jig: `profiles/jig/skills/view-contract-enforcer/SKILL.md`
- Vars: `view_name`, `http_method`, `request_schema_name`, `response_schema_name`, `service_symbol`, `url_path`, `url_name`, `test_name`, `test_url`
- Focus: request/response contract + route + service handoff

3. `query-layer-discipline`
- Control: `profiles/control/skills/query-layer-discipline/SKILL.md`
- Jig: `profiles/jig/skills/query-layer-discipline/SKILL.md`
- Vars: `model_name`, `queryset_name`, `manager_name`, `selector_name`, `selector_file`, `view_name`
- Focus: queryset/manager/selector read-path structure

4. `deterministic-service-test`
- Control: `profiles/control/skills/deterministic-service-test/SKILL.md`
- Jig: `profiles/jig/skills/deterministic-service-test/SKILL.md`
- Vars: `service_symbol`, `module_path`, `create_method`, `cancel_method`
- Focus: deterministic tests and `# Act` structure

5. `structured-logging-contract`
- Control: `profiles/control/skills/structured-logging-contract/SKILL.md`
- Jig: `profiles/jig/skills/structured-logging-contract/SKILL.md`
- Vars: `target_file`, `function_name`, `event_namespace`, `step_name`, `entity_id_expr`
- Focus: stable structured log naming and payload keys

## Runner Guidance

- Use the same scenario and prompt for both arms.
- In directed prompts, name one skill from this list explicitly.
- Keep variable names identical across arms to isolate implementation effects.
