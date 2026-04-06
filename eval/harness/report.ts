import type { TrialResult, AggregateScores } from "./types.ts";

export function aggregate(results: TrialResult[]): AggregateScores {
  if (results.length === 0) {
    return {
      overall_assertion: 0,
      jig_used_pct: 0,
      by_agent: {},
      by_tier: {},
      by_prompt_tier: {},
      by_category: {},
      weakest_scenarios: [],
    };
  }

  const overall_assertion = mean(results.map((r) => r.scores.assertion_score));
  const jigResults = results.filter((r) => r.mode === "jig");
  const jig_used_pct = jigResults.length > 0
    ? jigResults.filter((r) => r.scores.jig_used).length / jigResults.length
    : 0;

  // By agent
  const by_agent: Record<string, number> = {};
  for (const [name, group] of groupBy(results, (r) => r.agent)) {
    by_agent[name] = mean(group.map((r) => r.scores.total));
  }

  // By tier
  const by_tier: Record<string, number> = {};
  for (const [name, group] of groupBy(results, (r) => r.tier ?? "unknown")) {
    by_tier[name] = mean(group.map((r) => r.scores.total));
  }

  // By prompt tier
  const by_prompt_tier: Record<string, number> = {};
  for (const [name, group] of groupBy(results, (r) => r.prompt_tier ?? "natural")) {
    by_prompt_tier[name] = mean(group.map((r) => r.scores.total));
  }

  // By category
  const by_category: Record<string, number> = {};
  for (const [name, group] of groupBy(results, (r) => r.category ?? "unknown")) {
    by_category[name] = mean(group.map((r) => r.scores.total));
  }

  // Weakest scenarios
  const byScenario = new Map<string, number[]>();
  for (const r of results) {
    if (!byScenario.has(r.scenario)) byScenario.set(r.scenario, []);
    byScenario.get(r.scenario)!.push(r.scores.total);
  }
  const scenarioScores = [...byScenario.entries()]
    .map(([name, scores]) => ({ name, score: mean(scores) }))
    .sort((a, b) => a.score - b.score);
  const weakest_scenarios = scenarioScores.slice(0, 3);

  // Duration stats
  const jigTrials = results.filter((r) => r.mode === "jig");
  const baselineTrials = results.filter((r) => r.mode === "baseline");
  const mean_duration_jig = jigTrials.length > 0 ? mean(jigTrials.map((r) => r.duration_ms)) : undefined;
  const mean_duration_baseline = baselineTrials.length > 0 ? mean(baselineTrials.map((r) => r.duration_ms)) : undefined;

  // Token/cost stats
  const mean_tokens_jig = jigTrials.length > 0 ? mean(jigTrials.map((r) => r.tokens_used ?? 0)) : undefined;
  const mean_tokens_baseline = baselineTrials.length > 0 ? mean(baselineTrials.map((r) => r.tokens_used ?? 0)) : undefined;
  const mean_input_tokens_jig = jigTrials.length > 0 ? mean(jigTrials.map((r) => r.input_tokens ?? 0)) : undefined;
  const mean_input_tokens_baseline = baselineTrials.length > 0 ? mean(baselineTrials.map((r) => r.input_tokens ?? 0)) : undefined;
  const mean_output_tokens_jig = jigTrials.length > 0 ? mean(jigTrials.map((r) => r.output_tokens ?? 0)) : undefined;
  const mean_output_tokens_baseline = baselineTrials.length > 0 ? mean(baselineTrials.map((r) => r.output_tokens ?? 0)) : undefined;
  const mean_cost_jig = jigTrials.length > 0 ? mean(jigTrials.map((r) => r.cost_usd ?? 0)) : undefined;
  const mean_cost_baseline = baselineTrials.length > 0 ? mean(baselineTrials.map((r) => r.cost_usd ?? 0)) : undefined;

  // Baseline delta
  let baseline_delta: number | undefined;
  if (jigTrials.length > 0 && baselineTrials.length > 0) {
    const jigMean = mean(jigTrials.map((r) => r.scores.total));
    const baselineMean = mean(baselineTrials.map((r) => r.scores.total));
    baseline_delta = jigMean - baselineMean;
  }

  return {
    overall_assertion,
    jig_used_pct,
    baseline_delta,
    by_agent,
    by_tier,
    by_prompt_tier,
    by_category,
    weakest_scenarios,
    mean_duration_jig,
    mean_duration_baseline,
    mean_tokens_jig,
    mean_tokens_baseline,
    mean_input_tokens_jig,
    mean_input_tokens_baseline,
    mean_output_tokens_jig,
    mean_output_tokens_baseline,
    mean_cost_jig,
    mean_cost_baseline,
  };
}

export function generateReport(results: TrialResult[]): string {
  const agg = aggregate(results);
  const lines: string[] = [];

  lines.push("=== Eval Report ===");
  lines.push("");
  lines.push(`Trials: ${results.length}`);
  lines.push(`Overall assertion score: ${agg.overall_assertion.toFixed(3)}`);
  lines.push(`Jig used: ${(agg.jig_used_pct * 100).toFixed(1)}%`);

  if (agg.baseline_delta != null) {
    lines.push(`Baseline delta (jig - baseline): ${agg.baseline_delta > 0 ? "+" : ""}${agg.baseline_delta.toFixed(3)}`);
  }

  lines.push("");
  lines.push("--- By Agent ---");
  for (const [name, score] of Object.entries(agg.by_agent)) {
    lines.push(`  ${name}: ${score.toFixed(3)}`);
  }

  lines.push("");
  lines.push("--- By Tier ---");
  for (const [name, score] of Object.entries(agg.by_tier)) {
    lines.push(`  ${name}: ${score.toFixed(3)}`);
  }

  if (Object.keys(agg.by_prompt_tier).length > 0) {
    lines.push("");
    lines.push("--- By Prompt Tier ---");
    for (const [name, score] of Object.entries(agg.by_prompt_tier)) {
      lines.push(`  ${name}: ${score.toFixed(3)}`);
    }
  }

  if (Object.keys(agg.by_category).length > 0) {
    lines.push("");
    lines.push("--- By Category ---");
    for (const [name, score] of Object.entries(agg.by_category)) {
      lines.push(`  ${name}: ${score.toFixed(3)}`);
    }
  }

  if (agg.weakest_scenarios.length > 0) {
    lines.push("");
    lines.push("--- Weakest Scenarios ---");
    for (const s of agg.weakest_scenarios) {
      lines.push(`  ${s.name}: ${s.score.toFixed(3)}`);
    }
  }

  if (agg.mean_duration_jig != null || agg.mean_tokens_jig != null) {
    lines.push("");
    lines.push("--- Efficiency ---");
    if (agg.mean_duration_jig != null) lines.push(`  Duration (jig): ${(agg.mean_duration_jig / 1000).toFixed(1)}s`);
    if (agg.mean_duration_baseline != null) lines.push(`  Duration (baseline): ${(agg.mean_duration_baseline / 1000).toFixed(1)}s`);
    if (agg.mean_tokens_jig != null) lines.push(`  Tokens (jig): ${Math.round(agg.mean_tokens_jig).toLocaleString()}`);
    if (agg.mean_tokens_baseline != null) lines.push(`  Tokens (baseline): ${Math.round(agg.mean_tokens_baseline).toLocaleString()}`);
    if (agg.mean_input_tokens_jig != null) lines.push(`  Input tokens (jig): ${Math.round(agg.mean_input_tokens_jig).toLocaleString()}`);
    if (agg.mean_input_tokens_baseline != null) lines.push(`  Input tokens (baseline): ${Math.round(agg.mean_input_tokens_baseline).toLocaleString()}`);
    if (agg.mean_output_tokens_jig != null) lines.push(`  Output tokens (jig): ${Math.round(agg.mean_output_tokens_jig).toLocaleString()}`);
    if (agg.mean_output_tokens_baseline != null) lines.push(`  Output tokens (baseline): ${Math.round(agg.mean_output_tokens_baseline).toLocaleString()}`);
    if (agg.mean_cost_jig != null) lines.push(`  Cost (jig): $${agg.mean_cost_jig.toFixed(4)}`);
    if (agg.mean_cost_baseline != null) lines.push(`  Cost (baseline): $${agg.mean_cost_baseline.toFixed(4)}`);
    if (agg.mean_tokens_jig != null && agg.mean_tokens_baseline != null && agg.mean_tokens_baseline > 0) {
      const saved = ((1 - agg.mean_tokens_jig / agg.mean_tokens_baseline) * 100).toFixed(1);
      lines.push(`  Token savings: ${saved}%`);
    }
    if (agg.mean_output_tokens_jig != null && agg.mean_output_tokens_baseline != null && agg.mean_output_tokens_baseline > 0) {
      const saved = ((1 - agg.mean_output_tokens_jig / agg.mean_output_tokens_baseline) * 100).toFixed(1);
      lines.push(`  Output token savings: ${saved}%`);
    }
    if (agg.mean_cost_jig != null && agg.mean_cost_baseline != null && agg.mean_cost_baseline > 0) {
      const saved = ((1 - agg.mean_cost_jig / agg.mean_cost_baseline) * 100).toFixed(1);
      lines.push(`  Cost savings: ${saved}%`);
    }
  }

  // Stddev per scenario-agent when reps > 1
  const combos = new Map<string, number[]>();
  for (const r of results) {
    const key = `${r.scenario}|${r.agent}|${r.prompt_tier ?? "natural"}`;
    if (!combos.has(key)) combos.set(key, []);
    combos.get(key)!.push(r.scores.total);
  }
  const multiRep = [...combos.entries()].filter(([, v]) => v.length > 1);
  if (multiRep.length > 0) {
    lines.push("");
    lines.push("--- Per Scenario-Agent-PromptTier (mean +/- stddev) ---");
    for (const [key, scores] of multiRep) {
      const [scenario, agent, promptTier] = key.split("|");
      const m = mean(scores);
      const sd = stddev(scores);
      lines.push(`  ${scenario} x ${agent} [${promptTier}]: ${m.toFixed(3)} +/- ${sd.toFixed(3)}`);
    }
  }

  return lines.join("\n");
}

export function generateMetricsOnly(results: TrialResult[]): string {
  const agg = aggregate(results);
  const lines: string[] = [];

  lines.push(`METRIC overall_assertion=${agg.overall_assertion.toFixed(3)}`);
  lines.push(`METRIC jig_used_pct=${agg.jig_used_pct.toFixed(3)}`);

  if (agg.baseline_delta != null) {
    lines.push(`METRIC baseline_delta=${agg.baseline_delta.toFixed(3)}`);
  }

  for (const [name, score] of Object.entries(agg.by_agent)) {
    lines.push(`METRIC agent.${name}=${score.toFixed(3)}`);
  }

  for (const [name, score] of Object.entries(agg.by_prompt_tier)) {
    lines.push(`METRIC prompt_tier.${name}=${score.toFixed(3)}`);
  }

  if (agg.mean_tokens_jig != null) lines.push(`METRIC mean_tokens_jig=${Math.round(agg.mean_tokens_jig)}`);
  if (agg.mean_tokens_baseline != null) lines.push(`METRIC mean_tokens_baseline=${Math.round(agg.mean_tokens_baseline)}`);
  if (agg.mean_cost_jig != null) lines.push(`METRIC mean_cost_jig=${agg.mean_cost_jig.toFixed(4)}`);
  if (agg.mean_cost_baseline != null) lines.push(`METRIC mean_cost_baseline=${agg.mean_cost_baseline.toFixed(4)}`);

  return lines.join("\n");
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

function groupBy<T>(items: T[], key: (item: T) => string): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const item of items) {
    const k = key(item);
    if (!map.has(k)) map.set(k, []);
    map.get(k)!.push(item);
  }
  return map;
}
