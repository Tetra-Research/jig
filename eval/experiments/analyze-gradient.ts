import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { readResults, ResultSchemaError, formatDiagnosticsSummary } from "../harness/results.ts";
import type { TrialResult, SchemaPolicyMode } from "../harness/types.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EVAL_ROOT = path.resolve(__dirname, "..");

const { values: args } = parseArgs({
  options: {
    format: { type: "string", default: "table" },
    results: { type: "string", default: path.join(EVAL_ROOT, "results", "results.jsonl") },
    "schema-mode": { type: "string", default: "compat" },
  },
  strict: true,
});

const LEVEL_NAMES = ["Control", "Skills Only", "Nudge", "Directed"];
const schemaMode = args["schema-mode"] as SchemaPolicyMode;
const VALID_SCHEMA_MODES: SchemaPolicyMode[] = ["strict", "compat"];

if (!VALID_SCHEMA_MODES.includes(schemaMode)) {
  console.error(`Unknown --schema-mode "${schemaMode}". Use: ${VALID_SCHEMA_MODES.join(", ")}`);
  process.exit(1);
}

function deriveLevel(r: TrialResult): number {
  if (r.skills_available === false) return 0;
  if (r.claude_md === "none") return 1;
  if (r.prompt_tier === "directed") return 3;
  return 2;
}

/** Total context consumed regardless of caching */
function effectiveInput(r: TrialResult): number | undefined {
  if (!hasCompleteEfficiencyMetrics(r)) return undefined;
  return r.input_tokens + r.cache_creation_input_tokens + r.cache_read_input_tokens;
}

function mean(values: number[]): number {
  if (values.length === 0) return 0;
  return values.reduce((a, b) => a + b, 0) / values.length;
}

function stddev(values: number[]): number {
  if (values.length < 2) return 0;
  const m = mean(values);
  const variance = values.reduce((sum, v) => sum + (v - m) ** 2, 0) / (values.length - 1);
  return Math.sqrt(variance);
}

interface LevelStats {
  level: number;
  name: string;
  trials: number;
  efficiency_covered: number;
  mean_score: number;
  stddev_score: number;
  jig_used_pct: number;
  mean_input?: number;
  mean_output?: number;
  mean_total?: number;
  mean_cost?: number;
  mean_duration_s: number;
}

function computeLevelStats(level: number, trials: TrialResult[]): LevelStats {
  const scores = trials.map((r) => r.scores.total);
  const jigUsed = trials.filter((r) => r.scores.jig_used).length;
  const efficiencyTrials = trials.filter(hasCompleteEfficiencyMetrics);
  const effectiveInputs = efficiencyTrials.map((r) => effectiveInput(r)!);
  return {
    level,
    name: LEVEL_NAMES[level] ?? `Level ${level}`,
    trials: trials.length,
    efficiency_covered: efficiencyTrials.length,
    mean_score: mean(scores),
    stddev_score: stddev(scores),
    jig_used_pct: trials.length > 0 ? jigUsed / trials.length : 0,
    mean_input: efficiencyTrials.length > 0 ? mean(effectiveInputs) : undefined,
    mean_output: efficiencyTrials.length > 0 ? mean(efficiencyTrials.map((r) => r.output_tokens!)) : undefined,
    mean_total: efficiencyTrials.length > 0 ? mean(efficiencyTrials.map((r) => r.tokens_used!)) : undefined,
    mean_cost: efficiencyTrials.length > 0 ? mean(efficiencyTrials.map((r) => r.cost_usd!)) : undefined,
    mean_duration_s: mean(trials.map((r) => r.duration_ms / 1000)),
  };
}

// Read and group results
let loadedResults;
try {
  loadedResults = readResults(args.results!, { schemaMode });
} catch (err) {
  if (err instanceof ResultSchemaError) {
    console.error(err.message);
    for (const line of formatDiagnosticsSummary(err.diagnostics)) {
      console.error(line);
    }
    process.exit(1);
  }
  throw err;
}

if (schemaMode === "compat") {
  for (const line of formatDiagnosticsSummary(loadedResults.diagnostics)) {
    console.error(line);
  }
}

const results = loadedResults.results;
if (results.length === 0) {
  console.error("No results found.");
  process.exit(1);
}

const byLevel = new Map<number, TrialResult[]>();
for (const r of results) {
  const level = deriveLevel(r);
  if (!byLevel.has(level)) byLevel.set(level, []);
  byLevel.get(level)!.push(r);
}

const stats = [0, 1, 2, 3]
  .filter((l) => byLevel.has(l))
  .map((l) => computeLevelStats(l, byLevel.get(l)!));

if (args.format === "table") {
  // Summary table
  console.log("| Level | Name | Trials | Score | Stddev | Jig% | Input | Output | Cost | Duration |");
  console.log("|-------|------|--------|-------|--------|------|-------|--------|------|----------|");
  for (const s of stats) {
    console.log(
      `| ${s.level} | ${s.name} | ${s.trials} | ${s.mean_score.toFixed(3)} | ${s.stddev_score.toFixed(3)} | ${(s.jig_used_pct * 100).toFixed(0)}% | ${formatIntWithCoverage(s.mean_input, s.efficiency_covered, s.trials)} | ${formatIntWithCoverage(s.mean_output, s.efficiency_covered, s.trials)} | ${formatCostWithCoverage(s.mean_cost, s.efficiency_covered, s.trials)} | ${s.mean_duration_s.toFixed(1)}s |`
    );
  }

  // Delta from control
  if (stats.length >= 2) {
    const control = stats[0];
    console.log("");
    console.log("### Delta from Control (Level 0)");
    console.log("| Level | Name | Score Δ | Input Δ | Output Δ | Cost Δ |");
    console.log("|-------|------|---------|---------|----------|--------|");
    for (const s of stats.slice(1)) {
      const scoreDelta = s.mean_score - control.mean_score;
      const inputDelta = percentDelta(s.mean_input, control.mean_input);
      const outputDelta = percentDelta(s.mean_output, control.mean_output);
      const costDelta = percentDelta(s.mean_cost, control.mean_cost);
      const fmt = (v: number) => `${v > 0 ? "+" : ""}${v.toFixed(1)}%`;
      console.log(
        `| ${s.level} | ${s.name} | ${scoreDelta > 0 ? "+" : ""}${scoreDelta.toFixed(3)} | ${inputDelta == null ? "N/A" : fmt(inputDelta)} | ${outputDelta == null ? "N/A" : fmt(outputDelta)} | ${costDelta == null ? "N/A" : fmt(costDelta)} |`
      );
    }
  }
} else if (args.format === "csv") {
  // CSV with one row per trial — raw fields for external analysis
  console.log("level,level_name,scenario,agent,rep,score,jig_used,input_tokens,output_tokens,cache_creation_tokens,cache_read_tokens,effective_input,tokens_total,cost_usd,duration_s");
  for (const r of results) {
    const level = deriveLevel(r);
    const ei = effectiveInput(r);
    console.log(
      `${level},${LEVEL_NAMES[level]},${r.scenario},${r.agent},${r.rep},${r.scores.total.toFixed(3)},${r.scores.jig_used},${fmtCsvNum(r.input_tokens)},${fmtCsvNum(r.output_tokens)},${fmtCsvNum(r.cache_creation_input_tokens)},${fmtCsvNum(r.cache_read_input_tokens)},${fmtCsvNum(ei)},${fmtCsvNum(r.tokens_used)},${fmtCsvCost(r.cost_usd)},${(r.duration_ms / 1000).toFixed(1)}`
    );
  }
} else if (args.format === "scenario") {
  // Per-scenario breakdown by level
  const scenarios = [...new Set(results.map((r) => r.scenario))].sort();
  const header = ["Scenario", ...stats.map((s) => `L${s.level} (${s.name})`)];
  console.log("| " + header.join(" | ") + " |");
  console.log("| " + header.map(() => "---").join(" | ") + " |");

  for (const scenario of scenarios) {
    const cells = [scenario];
    for (const s of stats) {
      const trials = (byLevel.get(s.level) ?? []).filter((r) => r.scenario === scenario);
      if (trials.length === 0) {
        cells.push("—");
      } else {
        const score = mean(trials.map((r) => r.scores.total));
        const efficiencyTrials = trials.filter(hasCompleteEfficiencyMetrics);
        const coverage = `${efficiencyTrials.length}/${trials.length}`;
        const out = efficiencyTrials.length > 0
          ? `${Math.round(mean(efficiencyTrials.map((r) => r.output_tokens!))).toLocaleString()} out`
          : "N/A out";
        const jig = trials.some((r) => r.scores.jig_used) ? "*" : "";
        cells.push(`${score.toFixed(2)} (${out}, cov ${coverage})${jig}`);
      }
    }
    console.log("| " + cells.join(" | ") + " |");
  }
  console.log("\n\\* = jig used in at least one trial");
} else {
  console.error(`Unknown format: ${args.format}. Use: table, csv, scenario`);
  process.exit(1);
}

function hasCompleteEfficiencyMetrics(r: TrialResult): boolean {
  return typeof r.input_tokens === "number" &&
    typeof r.output_tokens === "number" &&
    typeof r.cache_creation_input_tokens === "number" &&
    typeof r.cache_read_input_tokens === "number" &&
    typeof r.tokens_used === "number" &&
    typeof r.cost_usd === "number";
}

function formatIntWithCoverage(value: number | undefined, covered: number, total: number): string {
  if (value == null) return `N/A (${covered}/${total})`;
  if (covered < total) return `${Math.round(value).toLocaleString()} (${covered}/${total})`;
  return Math.round(value).toLocaleString();
}

function formatCostWithCoverage(value: number | undefined, covered: number, total: number): string {
  if (value == null) return `N/A (${covered}/${total})`;
  if (covered < total) return `$${value.toFixed(2)} (${covered}/${total})`;
  return `$${value.toFixed(2)}`;
}

function percentDelta(current: number | undefined, control: number | undefined): number | undefined {
  if (current == null || control == null || control === 0) return undefined;
  return ((current - control) / control) * 100;
}

function fmtCsvNum(value: number | undefined): string {
  if (value == null) return "";
  return String(value);
}

function fmtCsvCost(value: number | undefined): string {
  if (value == null) return "";
  return value.toFixed(4);
}
