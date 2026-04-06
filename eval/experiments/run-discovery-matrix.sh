#!/usr/bin/env bash
set -euo pipefail

# Full discovery matrix runner with sharded result files.
#
# Factors:
# - mode: baseline, jig
# - prompt tier: natural, ambient (plus directed in jig mode)
# - CLAUDE.md: none, empty, shared
# - strip skills: 0, 1
#
# Usage:
#   REPS=1 AGENT=claude-code JOBS=8 bash experiments/run-discovery-matrix.sh
#   INCLUDE_PARALLEL_CONTROL=1 REPS=1 AGENT=claude-code JOBS=8 bash experiments/run-discovery-matrix.sh
#   DRY_RUN=1 SCENARIO=add-view REPS=1 AGENT=claude-code JOBS=4 bash experiments/run-discovery-matrix.sh

REPS="${REPS:-1}"
AGENT="${AGENT:-claude-code}"
JOBS="${JOBS:-6}"
SCHEMA_MODE="${SCHEMA_MODE:-strict}"
SCENARIO="${SCENARIO:-}"
DRY_RUN="${DRY_RUN:-0}"
INCLUDE_PARALLEL_CONTROL="${INCLUDE_PARALLEL_CONTROL:-0}"

cd "$(dirname "$0")/.."

TIMESTAMP="$(date +%Y%m%dT%H%M%S)"
SCENARIO_FLAG=()
if [ -n "$SCENARIO" ]; then
  SCENARIO_FLAG=(--scenario "$SCENARIO")
fi

if ! [[ "$JOBS" =~ ^[0-9]+$ ]] || [ "$JOBS" -lt 1 ]; then
  echo "ERROR: JOBS must be a positive integer (got: $JOBS)" >&2
  exit 1
fi

run_matrix_once() {
  local label="$1"
  local jobs="$2"
  local run_id="${TIMESTAMP}-${label}"
  local shard_dir="results/tmp/discovery-matrix-${run_id}"
  local archive_path="results/archive/results-discovery-matrix-${run_id}.jsonl"
  local manifest_path="results/archive/results-discovery-matrix-${run_id}.manifest.tsv"
  local command_file="$shard_dir/commands.txt"

  mkdir -p "$shard_dir" "results/archive"
  : > "$command_file"
  : > "$manifest_path"
  printf "cell\tmode\tprompt_tier\tclaude_md\tstrip_skills\tresults_path\n" > "$manifest_path"

  local tiers
  local total_cells=0

  for mode in baseline jig; do
    if [ "$mode" = "baseline" ]; then
      tiers="natural ambient"
    else
      tiers="natural ambient directed"
    fi

    for prompt_tier in $tiers; do
      for claude_md in none empty shared; do
        for strip in 0 1; do
          local cell="${mode}-pt-${prompt_tier}-cmd-${claude_md}-skills-${strip}"
          local shard_file="$shard_dir/${cell}.jsonl"
          local strip_flag=""
          if [ "$strip" = "1" ]; then
            strip_flag="--strip-skills"
          fi
          local dry_flag=""
          if [ "$DRY_RUN" = "1" ]; then
            dry_flag="--dry-run"
          fi

          # shellcheck disable=SC2145
          local cmd="npx tsx harness/run.ts --mode \"$mode\" --prompt-tier \"$prompt_tier\" --claude-md \"$claude_md\" $strip_flag --agent \"$AGENT\" --reps \"$REPS\" --schema-mode \"$SCHEMA_MODE\" --results \"$shard_file\" $dry_flag ${SCENARIO_FLAG[*]}"
          echo "$cmd" >> "$command_file"
          printf "%s\t%s\t%s\t%s\t%s\t%s\n" "$cell" "$mode" "$prompt_tier" "$claude_md" "$strip" "$shard_file" >> "$manifest_path"
          total_cells=$((total_cells + 1))
        done
      done
    done
  done

  echo "=== Discovery Matrix ($run_id) ==="
  echo "Cells: $total_cells | Reps: $REPS | Agent: $AGENT | Jobs: $jobs | Schema mode: $SCHEMA_MODE | Scenario: ${SCENARIO:-all} | Dry run: $DRY_RUN"
  echo "Shard dir: $shard_dir"

  set +e
  xargs -P "$jobs" -I CMD bash -lc "CMD" < "$command_file"
  local status=$?
  set -e
  if [ "$status" -ne 0 ]; then
    echo "WARNING: one or more matrix cells failed (exit $status)" >&2
  fi

  if [ "$DRY_RUN" = "1" ]; then
    echo "Dry run complete. Commands: $command_file"
    echo "Manifest: $manifest_path"
    return
  fi

  : > "$archive_path"
  local shard_count=0
  for f in "$shard_dir"/*.jsonl; do
    [ -f "$f" ] || continue
    cat "$f" >> "$archive_path"
    shard_count=$((shard_count + 1))
  done

  echo "Merged $shard_count shard file(s) into: $archive_path"
  echo "Manifest: $manifest_path"
  echo ""
  echo "Quick analysis:"
  npx tsx experiments/analyze-gradient.ts --results "$archive_path" --schema-mode "$SCHEMA_MODE" || true
  echo ""
}

if [ "$INCLUDE_PARALLEL_CONTROL" = "1" ] && [ "$JOBS" -gt 1 ]; then
  run_matrix_once "seq-j1" 1
  run_matrix_once "par-j${JOBS}" "$JOBS"
else
  run_matrix_once "j${JOBS}" "$JOBS"
fi

echo "Done."
