#!/usr/bin/env bash
set -euo pipefail

# Gradient experiment: 4 levels of skill guidance
# Usage: REPS=3 AGENT=claude-code bash experiments/run-gradient.sh
# Single scenario: SCENARIOS=add-view REPS=1 bash experiments/run-gradient.sh
# Sequential mode: PARALLEL=0 bash experiments/run-gradient.sh
# Custom results path: RESULTS_PATH=results/archive/my-run.jsonl bash experiments/run-gradient.sh

REPS="${REPS:-1}"
AGENT="${AGENT:-claude-code}"
SCENARIOS="${SCENARIOS:-}"
PARALLEL="${PARALLEL:-1}"
SCHEMA_MODE="${SCHEMA_MODE:-compat}"
RESULTS_PATH="${RESULTS_PATH:-results/results.jsonl}"

cd "$(dirname "$0")/.."

SCENARIO_FLAG=""
if [ -n "$SCENARIOS" ]; then
  SCENARIO_FLAG="--scenario $SCENARIOS"
fi

TIMESTAMP=$(date +%Y%m%dT%H%M%S)
echo "=== Gradient Experiment ($TIMESTAMP) ==="
echo "Reps: $REPS | Agent: $AGENT | Scenarios: ${SCENARIOS:-all} | Parallel: $PARALLEL | Schema mode: $SCHEMA_MODE"
echo "Results path: $RESULTS_PATH"
echo ""

run_level() {
  local level=$1 label=$2
  shift 2
  echo "--- Level $level: $label ---"
  npx tsx harness/run.ts "$@" --schema-mode "$SCHEMA_MODE" --agent "$AGENT" --reps "$REPS" --results "$RESULTS_PATH" $SCENARIO_FLAG 2>&1
  echo "--- Level $level complete ---"
}

if [ "$PARALLEL" = "1" ]; then
  # Run all 4 levels in parallel — each writes to the same JSONL (append-safe)
  run_level 0 "Control (no skills, no CLAUDE.md)" \
    --mode baseline --strip-skills --claude-md none --disable-jig-binary --prompt-tier natural &
  PID0=$!

  run_level 1 "Skills Only (skills present, no CLAUDE.md)" \
    --claude-md none --prompt-tier natural &
  PID1=$!

  run_level 2 "Nudge (skills + CLAUDE.md)" \
    --claude-md shared --prompt-tier natural &
  PID2=$!

  run_level 3 "Directed (skills + CLAUDE.md + directed prompt)" \
    --claude-md shared --prompt-tier directed &
  PID3=$!

  # Wait for all and capture exit codes
  FAILED=0
  for pid in $PID0 $PID1 $PID2 $PID3; do
    wait "$pid" || FAILED=$((FAILED + 1))
  done

  if [ "$FAILED" -gt 0 ]; then
    echo "WARNING: $FAILED level(s) had failures"
  fi
else
  # Sequential mode
  run_level 0 "Control (no skills, no CLAUDE.md)" \
    --mode baseline --strip-skills --claude-md none --disable-jig-binary --prompt-tier natural

  echo ""
  run_level 1 "Skills Only (skills present, no CLAUDE.md)" \
    --claude-md none --prompt-tier natural

  echo ""
  run_level 2 "Nudge (skills + CLAUDE.md)" \
    --claude-md shared --prompt-tier natural

  echo ""
  run_level 3 "Directed (skills + CLAUDE.md + directed prompt)" \
    --claude-md shared --prompt-tier directed
fi

echo ""
echo "=== Gradient complete ==="
echo "Results: $RESULTS_PATH"
echo "Analyze: npx tsx experiments/analyze-gradient.ts"
