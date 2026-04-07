#!/usr/bin/env bash
set -euo pipefail

# Example:
# SCENARIO=add-field-v9-template-first REPS=3 AGENT=claude-code \
# CONTROL_PROFILE=head2head/profiles/control \
# JIG_PROFILE=head2head/profiles/jig \
# bash experiments/run-head2head.sh

SCENARIO="${SCENARIO:-}"
REPS="${REPS:-1}"
AGENT="${AGENT:-claude-code}"
PROMPT_SOURCE="${PROMPT_SOURCE:-natural}"
CONTROL_PROFILE="${CONTROL_PROFILE:-head2head/profiles/control}"
JIG_PROFILE="${JIG_PROFILE:-head2head/profiles/jig}"
THINKING_MODE="${THINKING_MODE:-1}"
DRY_RUN="${DRY_RUN:-0}"
RESULTS_PATH="${RESULTS_PATH:-results/head2head-results.jsonl}"
PAIRS_PATH="${PAIRS_PATH:-results/head2head-pairs.jsonl}"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-results/head2head-artifacts}"

if [ -z "$SCENARIO" ]; then
  echo "SCENARIO is required."
  echo "Example: SCENARIO=add-field-v9-template-first bash experiments/run-head2head.sh"
  exit 1
fi

cd "$(dirname "$0")/.."

THINKING_FLAG=()
if [ "$THINKING_MODE" = "1" ]; then
  THINKING_FLAG=(--thinking-mode)
fi

DRY_RUN_FLAG=()
if [ "$DRY_RUN" = "1" ]; then
  DRY_RUN_FLAG=(--dry-run)
fi

echo "=== Head-to-Head Run ==="
echo "Scenario: $SCENARIO"
echo "Agent: $AGENT"
echo "Reps: $REPS"
echo "Prompt source: $PROMPT_SOURCE"
echo "Control profile: $CONTROL_PROFILE"
echo "Jig profile: $JIG_PROFILE"
echo "Results: $RESULTS_PATH"
echo "Pairs: $PAIRS_PATH"
echo ""

node --import tsx head2head/run.ts \
  --scenario "$SCENARIO" \
  --agent "$AGENT" \
  --reps "$REPS" \
  --prompt-source "$PROMPT_SOURCE" \
  --control-profile "$CONTROL_PROFILE" \
  --jig-profile "$JIG_PROFILE" \
  --results "$RESULTS_PATH" \
  --pairs "$PAIRS_PATH" \
  --artifacts-dir "$ARTIFACTS_DIR" \
  "${DRY_RUN_FLAG[@]}" \
  "${THINKING_FLAG[@]}"
