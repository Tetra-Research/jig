import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { loadAllScenarios } from "../harness/scenarios.ts";
import { aggregateFileScore } from "../lib/diff.ts";

type HeadToHeadRow = {
  schema_version: string;
  scenario: string;
  rep: number;
  arm: "control" | "jig";
  scores: { total: number };
  file_score: number;
  assertions: Array<{ contains: string; passed: boolean }>;
  telemetry: {
    tool_calls_by_name?: Record<string, number>;
    result_event?: { result?: string };
  };
  agent_artifacts?: {
    dir: string;
    changed_files_manifest?: string;
  };
};

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EVAL_ROOT = path.resolve(__dirname, "..");

const { values: args } = parseArgs({
  options: {
    results: { type: "string", default: "results/head2head-results.jsonl" },
    output: { type: "string" },
    "top-examples": { type: "string", default: "5" },
  },
  strict: true,
});

const resultsPath = path.resolve(process.cwd(), args.results!);
const outputPath = args.output ? path.resolve(process.cwd(), args.output) : undefined;
const topExamples = Number.parseInt(args["top-examples"] ?? "5", 10);

const rows = readRows(resultsPath);
const scenarios = loadAllScenarios(path.join(EVAL_ROOT, "scenarios"));
const scenariosByName = new Map(scenarios.map((scenario) => [scenario.name, scenario]));

const grouped = new Map<string, HeadToHeadRow[]>();
for (const row of rows) {
  const bucket = grouped.get(row.scenario) ?? [];
  bucket.push(row);
  grouped.set(row.scenario, bucket);
}

const lines: string[] = [];
lines.push(`# Head-to-Head Adversarial Review`);
lines.push("");
lines.push(`- Results: \`${path.relative(process.cwd(), resultsPath)}\``);
lines.push(`- Rows analyzed: ${rows.length}`);
lines.push(`- Scenarios analyzed: ${grouped.size}`);
lines.push("");
lines.push(`## Scenario Summary`);
lines.push("");
lines.push(`| Scenario | No-op File Score | Control Pass | Control Edit Rate | Control Analysis-Only | Jig Pass |`);
lines.push(`| --- | ---: | ---: | ---: | ---: | ---: |`);

for (const scenarioName of [...grouped.keys()].sort()) {
  const scenarioRows = grouped.get(scenarioName)!;
  const control = scenarioRows.filter((row) => row.arm === "control");
  const jig = scenarioRows.filter((row) => row.arm === "jig");
  const scenario = scenariosByName.get(scenarioName);
  const noOpFileScore = scenario
    ? aggregateFileScore(scenario, path.join(scenario.scenarioDir, "codebase"))
    : 0;

  lines.push(
    `| ${scenarioName} | ${fmtPct(noOpFileScore)} | ${fmtPct(passRate(control))} | ${fmtPct(editRate(control))} | ${fmtPct(analysisOnlyRate(control))} | ${fmtPct(passRate(jig))} |`
  );
}

lines.push("");
lines.push(`## Key Findings`);
lines.push("");

for (const scenarioName of [...grouped.keys()].sort()) {
  const scenarioRows = grouped.get(scenarioName)!;
  const control = scenarioRows.filter((row) => row.arm === "control");
  const jig = scenarioRows.filter((row) => row.arm === "jig");
  const scenario = scenariosByName.get(scenarioName);
  const noOpFileScore = scenario
    ? aggregateFileScore(scenario, path.join(scenario.scenarioDir, "codebase"))
    : 0;

  if (analysisOnlyRate(control) >= 0.5) {
    lines.push(`- \`${scenarioName}\`: control often behaved as analysis-only (${countAnalysisOnly(control)}/${control.length}) instead of editing files.`);
  }
  if (noOpFileScore >= 0.25) {
    lines.push(`- \`${scenarioName}\`: no-op baseline already scores ${fmtPct(noOpFileScore)} on \`file_score\`, so raw file similarity overstates progress on untouched runs.`);
  }
  if (passRate(control) === 1 && mean(control.map((row) => row.file_score)) < 0.95) {
    lines.push(`- \`${scenarioName}\`: control reaches full assertion score but still diverges materially from expected file shape (mean file_score ${fmtPct(mean(control.map((row) => row.file_score)))}).`);
  }
  if (passRate(control) < passRate(jig) && editRate(control) > 0) {
    lines.push(`- \`${scenarioName}\`: control does edit in some or all runs, but its outputs are less consistent than jig-backed runs.`);
  }
}

const examples = collectExamples(rows, topExamples);
if (examples.length > 0) {
  lines.push("");
  lines.push(`## Comparison Examples`);
  lines.push("");

  for (const example of examples) {
    lines.push(`### ${example.scenario} rep ${example.rep}`);
    lines.push("");
    lines.push(`- Control: score=${example.control.scores.total.toFixed(2)}, file_score=${example.control.file_score.toFixed(2)}`);
    lines.push(`- Jig: score=${example.jig.scores.total.toFixed(2)}, file_score=${example.jig.file_score.toFixed(2)}`);
    lines.push(`- Control failed assertions: ${example.failedAssertions.length > 0 ? example.failedAssertions.join("; ") : "none"}`);
    lines.push(`- Control result: ${quoteInline(summarizeResult(example.control))}`);
    lines.push(`- Jig result: ${quoteInline(summarizeResult(example.jig))}`);
    if (example.control.agent_artifacts?.dir) {
      lines.push(`- Control artifacts: \`${path.relative(process.cwd(), example.control.agent_artifacts.dir)}\``);
    }
    if (example.jig.agent_artifacts?.dir) {
      lines.push(`- Jig artifacts: \`${path.relative(process.cwd(), example.jig.agent_artifacts.dir)}\``);
    }
    lines.push("");
  }
}

const report = lines.join("\n");
if (outputPath) {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, report, "utf-8");
}
console.log(report);

function readRows(filePath: string): HeadToHeadRow[] {
  const raw = fs.readFileSync(filePath, "utf-8");
  return raw
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => JSON.parse(line) as HeadToHeadRow)
    .filter((row) => row.schema_version === "head2head_v1");
}

function passRate(rows: HeadToHeadRow[]): number {
  if (rows.length === 0) return 0;
  return rows.filter((row) => row.scores.total === 1).length / rows.length;
}

function editRate(rows: HeadToHeadRow[]): number {
  if (rows.length === 0) return 0;
  return rows.filter(didEdit).length / rows.length;
}

function analysisOnlyRate(rows: HeadToHeadRow[]): number {
  if (rows.length === 0) return 0;
  return countAnalysisOnly(rows) / rows.length;
}

function countAnalysisOnly(rows: HeadToHeadRow[]): number {
  return rows.filter(isAnalysisOnly).length;
}

function didEdit(row: HeadToHeadRow): boolean {
  const changedFiles = readChangedFiles(row.agent_artifacts?.changed_files_manifest);
  if (changedFiles.length > 0) return true;
  const toolCounts = row.telemetry.tool_calls_by_name ?? {};
  return (toolCounts.Edit ?? 0) + (toolCounts.Write ?? 0) + (toolCounts.MultiEdit ?? 0) + (toolCounts.NotebookEdit ?? 0) > 0;
}

function isAnalysisOnly(row: HeadToHeadRow): boolean {
  if (didEdit(row)) return false;
  const result = summarizeResult(row).toLowerCase();
  return result.includes("no files created")
    || result.includes("checklist")
    || result.includes("review only")
    || result.includes("structure only")
    || result.includes("let me know if you want this written")
    || result.includes("applying the")
    || result.includes("plan for");
}

function readChangedFiles(filePath: string | undefined): string[] {
  if (!filePath || !fs.existsSync(filePath)) return [];
  return fs.readFileSync(filePath, "utf-8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function collectExamples(rows: HeadToHeadRow[], count: number) {
  const pairs = new Map<string, { control?: HeadToHeadRow; jig?: HeadToHeadRow }>();
  for (const row of rows) {
    const key = `${row.scenario}|${row.rep}`;
    const bucket = pairs.get(key) ?? {};
    bucket[row.arm] = row;
    pairs.set(key, bucket);
  }

  return [...pairs.entries()]
    .map(([key, pair]) => ({ key, ...pair }))
    .filter((pair): pair is { key: string; control: HeadToHeadRow; jig: HeadToHeadRow } => Boolean(pair.control && pair.jig))
    .filter((pair) => didEdit(pair.control))
    .filter((pair) => pair.control.scores.total < pair.jig.scores.total || pair.control.file_score < pair.jig.file_score)
    .sort((a, b) => {
      const aGap = (a.jig.scores.total - a.control.scores.total) + (a.jig.file_score - a.control.file_score);
      const bGap = (b.jig.scores.total - b.control.scores.total) + (b.jig.file_score - b.control.file_score);
      return bGap - aGap;
    })
    .slice(0, count)
    .map((pair) => ({
      scenario: pair.control.scenario,
      rep: pair.control.rep,
      control: pair.control,
      jig: pair.jig,
      failedAssertions: pair.control.assertions.filter((assertion) => !assertion.passed).map((assertion) => assertion.contains),
    }));
}

function summarizeResult(row: HeadToHeadRow): string {
  return (row.telemetry.result_event?.result ?? "").replace(/\s+/g, " ").trim();
}

function mean(values: number[]): number {
  if (values.length === 0) return 0;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function fmtPct(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}

function quoteInline(value: string): string {
  if (value.length === 0) return "`<empty>`";
  const truncated = value.length > 180 ? `${value.slice(0, 177)}...` : value;
  return `\`${truncated.replace(/`/g, "'")}\``;
}
