# Workstream Planning

Run dual-agent planning: Claude and Codex both produce execution plans from the same workstream context, then optionally synthesize into a merged plan.

## Usage
```
/ws-plan <workstream-name> [--synthesize] [--agent claude|codex|both]
```

## Arguments
$ARGUMENTS

---

## How It Works

This command runs `./scripts/ws-plan.sh $ARGUMENTS` which:

1. Builds a planning prompt from workstream docs (SPEC, PLAN, SHARED-CONTEXT, INVARIANTS, ARCHITECTURE)
2. Runs both Claude and Codex in parallel with the same prompt
3. Saves timestamped outputs to `docs/workstreams/<name>/exec/`
4. With `--synthesize`: merges both plans, marking agreements and `[HUMAN DECISION NEEDED]` points

## EARS Format

All acceptance criteria MUST use EARS (Easy Approach to Requirements Syntax):

| Type | Pattern | Example |
|------|---------|---------|
| **Ubiquitous** | The system SHALL `<response>` | The system SHALL log all API requests |
| **Event** | WHEN `<trigger>`, the system SHALL `<response>` | WHEN a user submits a form, the system SHALL validate all fields |
| **State** | WHILE `<state>`, the system SHALL `<response>` | WHILE the queue is full, the system SHALL reject new messages |
| **Option** | WHERE `<feature>`, the system SHALL `<response>` | WHERE SSO is enabled, the system SHALL redirect to the IdP |
| **Unwanted** | IF `<condition>`, the system SHALL `<response>` | IF the token is expired, the system SHALL return 401 |

## After Planning

1. Review both agent outputs in `exec/`
2. If not already synthesized, run again with `--synthesize`
3. Edit `exec/synthesized.md` — resolve any `[HUMAN DECISION NEEDED]` markers
4. Run `/ws-plan-review <name>` for adversarial review
