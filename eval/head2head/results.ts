import fs from "node:fs";
import path from "node:path";
import type {
  HeadToHeadPairResult,
  HeadToHeadTrialResult,
  NumericDelta,
} from "./types.ts";

export function writeHeadToHeadTrialResult(result: HeadToHeadTrialResult, filePath: string): void {
  appendJsonl(result, filePath);
}

export function writeHeadToHeadPairResult(result: HeadToHeadPairResult, filePath: string): void {
  appendJsonl(result, filePath);
}

export function buildPairResult(
  runId: string,
  scenario: string,
  rep: number,
  agent: string,
  control: HeadToHeadTrialResult,
  jig: HeadToHeadTrialResult
): HeadToHeadPairResult {
  return {
    schema_version: "head2head_pair_v1",
    timestamp: new Date().toISOString(),
    run_id: runId,
    scenario,
    rep,
    agent,
    score: makeDelta(control.scores.total, jig.scores.total),
    file_score: makeDelta(control.file_score, jig.file_score),
    duration_ms: makeDelta(control.duration_ms, jig.duration_ms),
    tool_calls: makeDelta(control.telemetry.tool_calls, jig.telemetry.tool_calls),
    context_tokens: makeDelta(control.telemetry.context_tokens, jig.telemetry.context_tokens),
    output_tokens: makeDelta(control.telemetry.output_tokens, jig.telemetry.output_tokens),
    tokens_used: makeDelta(control.telemetry.tokens_used, jig.telemetry.tokens_used),
    cost_usd: makeDelta(control.telemetry.cost_usd, jig.telemetry.cost_usd),
  };
}

export function summarizePairs(pairs: HeadToHeadPairResult[]): string {
  if (pairs.length === 0) {
    return "No complete control/jig pairs were produced.";
  }

  const lines: string[] = [];
  lines.push("=== Head-to-Head Summary ===");
  lines.push(`Pairs: ${pairs.length}`);
  lines.push("");

  const score = summarizeDelta(pairs.map((pair) => pair.score));
  if (score) {
    lines.push(`Score delta (jig-control): ${fmtFixed(score.meanAbs, 3)} (${fmtSignedPct(score.meanPct)})`);
  }

  const outputTokens = summarizeDelta(pairs.map((pair) => pair.output_tokens));
  if (outputTokens) {
    lines.push(`Output tokens delta: ${fmtSigned(outputTokens.meanAbs)} (${fmtSignedPct(outputTokens.meanPct)})`);
  }

  const contextTokens = summarizeDelta(pairs.map((pair) => pair.context_tokens));
  if (contextTokens) {
    lines.push(`Context tokens delta: ${fmtSigned(contextTokens.meanAbs)} (${fmtSignedPct(contextTokens.meanPct)})`);
  }

  const totalTokens = summarizeDelta(pairs.map((pair) => pair.tokens_used));
  if (totalTokens) {
    lines.push(`Total tokens delta: ${fmtSigned(totalTokens.meanAbs)} (${fmtSignedPct(totalTokens.meanPct)})`);
  }

  const cost = summarizeDelta(pairs.map((pair) => pair.cost_usd));
  if (cost) {
    lines.push(`Cost delta: ${fmtFixed(cost.meanAbs, 4)} (${fmtSignedPct(cost.meanPct)})`);
  }

  const duration = summarizeDelta(pairs.map((pair) => pair.duration_ms));
  if (duration) {
    lines.push(`Duration delta (ms): ${fmtSigned(duration.meanAbs)} (${fmtSignedPct(duration.meanPct)})`);
  }

  const toolCalls = summarizeDelta(pairs.map((pair) => pair.tool_calls));
  if (toolCalls) {
    lines.push(`Tool calls delta: ${fmtSigned(toolCalls.meanAbs)} (${fmtSignedPct(toolCalls.meanPct)})`);
  }

  return lines.join("\n");
}

function appendJsonl(value: unknown, filePath: string): void {
  try {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.appendFileSync(filePath, JSON.stringify(value) + "\n");
  } catch (err) {
    console.error(`[head2head] Failed writing JSONL row to ${filePath}: ${err}`);
  }
}

function makeDelta(control: number | undefined, jig: number | undefined): NumericDelta | undefined {
  if (control == null || jig == null) return undefined;
  const absDelta = jig - control;
  const pctDelta = control !== 0 ? (absDelta / control) * 100 : undefined;
  return {
    control,
    jig,
    abs_delta: absDelta,
    pct_delta: pctDelta,
  };
}

function summarizeDelta(values: Array<NumericDelta | undefined>): { meanAbs: number; meanPct?: number } | undefined {
  const present = values.filter((value): value is NumericDelta => value != null);
  if (present.length === 0) return undefined;

  const meanAbs = present.reduce((sum, value) => sum + value.abs_delta, 0) / present.length;
  const pctValues = present.filter((value) => typeof value.pct_delta === "number");
  const meanPct = pctValues.length > 0
    ? pctValues.reduce((sum, value) => sum + (value.pct_delta as number), 0) / pctValues.length
    : undefined;

  return { meanAbs, meanPct };
}

function fmtFixed(value: number, digits: number): string {
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(digits)}`;
}

function fmtSigned(value: number): string {
  const rounded = Math.round(value);
  const sign = rounded > 0 ? "+" : "";
  return `${sign}${rounded.toLocaleString()}`;
}

function fmtSignedPct(value: number | undefined): string {
  if (value == null || !Number.isFinite(value)) return "N/A";
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(1)}%`;
}
