import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";
import { loadAllScenarios, validateScenario } from "./scenarios.ts";
import { loadAgentConfigs, getAgentByName, invokeAgent } from "./agents.ts";
import { createSandbox } from "../lib/sandbox.ts";
import { scoreAssertions, scoreNegativeAssertions, scoreJigUsage, scoreEfficiency, computeTrialScore } from "./score.ts";
import { aggregateFileScore } from "../lib/diff.ts";
import { writeTrialResult, readResults } from "./results.ts";
import { transformPromptForBaseline } from "./baseline.ts";
import { generateReport, generateMetricsOnly } from "./report.ts";
import type { Scenario, AgentConfig, TrialResult } from "./types.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EVAL_ROOT = path.resolve(__dirname, "..");

const { values: args } = parseArgs({
  options: {
    scenario: { type: "string" },
    agent: { type: "string" },
    reps: { type: "string", default: "1" },
    tier: { type: "string" },
    mode: { type: "string", default: "jig" },
    "dry-run": { type: "boolean", default: false },
    "metrics-only": { type: "boolean", default: false },
  },
  strict: true,
});

const reps = parseInt(args.reps!, 10);
const mode = args.mode as "jig" | "baseline";

// Load scenarios
const scenariosDir = path.join(EVAL_ROOT, "scenarios");
let scenarios = loadAllScenarios(scenariosDir);

if (args.scenario) {
  const found = scenarios.find((s) => s.name === args.scenario);
  if (!found) {
    const available = scenarios.map((s) => s.name).join(", ");
    console.error(`Unknown scenario "${args.scenario}". Available: ${available}`);
    process.exit(1);
  }
  scenarios = [found];
}

if (args.tier) {
  scenarios = scenarios.filter((s) => s.tier === args.tier);
  if (scenarios.length === 0) {
    console.error(`No scenarios match tier "${args.tier}".`);
    process.exit(1);
  }
}

// Load agents
const agentsPath = path.join(EVAL_ROOT, "agents.yaml");
const allAgents = loadAgentConfigs(agentsPath);
let agents: AgentConfig[];

if (args.agent) {
  try {
    agents = [getAgentByName(allAgents, args.agent)];
  } catch (err) {
    console.error((err as Error).message);
    process.exit(1);
  }
} else {
  agents = allAgents;
}

// Validate all scenarios
let hasValidationErrors = false;
for (const scenario of scenarios) {
  const errors = validateScenario(scenario);
  if (errors.length > 0) {
    hasValidationErrors = true;
    for (const err of errors) {
      console.error(`[validate] ${scenario.name}: ${err.field} — ${err.message}`);
    }
  }
}

if (hasValidationErrors) {
  console.error("Validation failed. Fix errors above before running.");
  process.exit(1);
}

// Get jig version
let jigVersion = "unknown";
try {
  jigVersion = execSync("jig --version", { encoding: "utf-8", stdio: "pipe" }).trim();
} catch {}

// Dry run
if (args["dry-run"]) {
  const total = scenarios.length * agents.length * reps;
  console.error(`Dry run: ${scenarios.length} scenarios x ${agents.length} agents x ${reps} reps = ${total} trials`);

  const byTier: Record<string, number> = {};
  for (const s of scenarios) {
    byTier[s.tier] = (byTier[s.tier] ?? 0) + 1;
  }
  console.error("By tier:", JSON.stringify(byTier));
  console.error(`Mode: ${mode}`);
  console.error(`Jig version: ${jigVersion}`);
  console.error("Scenarios:");
  for (const s of scenarios) {
    console.error(`  - ${s.name} (${s.tier})`);
  }
  console.error("Agents:");
  for (const a of agents) {
    console.error(`  - ${a.name}`);
  }
  process.exit(0);
}

// Run trials
const resultsPath = path.join(EVAL_ROOT, "results", "results.jsonl");
const totalTrials = scenarios.length * agents.length * reps;
let trialNum = 0;

for (const scenario of scenarios) {
  for (const agent of agents) {
    for (let rep = 1; rep <= reps; rep++) {
      trialNum++;
      let sandbox;
      try {
        sandbox = await createSandbox(scenario);
      } catch (err) {
        console.error(`[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} rep=${rep}  SANDBOX FAILED: ${err}`);
        continue;
      }

      try {
        // Assemble prompt
        const prompt = mode === "baseline"
          ? transformPromptForBaseline(scenario)
          : (scenario.context ? `${scenario.context}\n\n${scenario.prompt}` : scenario.prompt);

        // Invoke agent
        const agentResult = await invokeAgent(agent, prompt, sandbox.workDir);

        // Score
        const assertionResults = scoreAssertions(scenario, sandbox.workDir);
        const negativeResults = scoreNegativeAssertions(scenario, sandbox.workDir);
        const fileSc = aggregateFileScore(scenario, sandbox.workDir);
        const agentOutput = agentResult.stdout + "\n" + agentResult.stderr;
        const jigUsage = scoreJigUsage(agentOutput, scenario);
        const efficiency = scoreEfficiency(agentResult.stdout);
        const trialScore = computeTrialScore(assertionResults, negativeResults, fileSc, jigUsage);

        const result: TrialResult = {
          scenario: scenario.name,
          agent: agent.name,
          mode,
          rep,
          timestamp: new Date().toISOString(),
          duration_ms: agentResult.durationMs,
          jig_version: sandbox.jigVersion || jigVersion,
          scores: trialScore,
          assertions: assertionResults,
          negative_assertions: negativeResults.results,
          jig_invocations: jigUsage.invocations,
          agent_exit_code: agentResult.exitCode,
          agent_tool_calls: efficiency.tool_calls,
          timeout: agentResult.timedOut,
          tags: scenario.tags ?? [],
        };

        writeTrialResult(result, resultsPath);

        const elapsed = (agentResult.durationMs / 1000).toFixed(1);
        console.error(
          `[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} rep=${rep}  score=${trialScore.total.toFixed(2)}  ${elapsed}s`
        );
      } catch (err) {
        console.error(`[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} rep=${rep}  ERROR: ${err}`);
      } finally {
        await sandbox.cleanup();
      }
    }
  }
}

// Report
const allResults = readResults(resultsPath);
if (allResults.length > 0) {
  if (args["metrics-only"]) {
    console.log(generateMetricsOnly(allResults));
  } else {
    console.error(generateReport(allResults));
    console.log(generateMetricsOnly(allResults));
  }
}
