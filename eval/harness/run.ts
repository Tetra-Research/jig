import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";
import { loadAllScenarios, validateScenario, VALID_PROMPT_TIERS } from "./scenarios.ts";
import { loadAgentConfigs, getAgentByName, invokeAgent } from "./agents.ts";
import { createSandbox } from "../lib/sandbox.ts";
import { scoreAssertions, scoreNegativeAssertions, scoreJigUsage, scoreEfficiency, computeTrialScore } from "./score.ts";
import { aggregateFileScore } from "../lib/diff.ts";
import { writeTrialResult, readResults } from "./results.ts";
import { transformPromptForBaseline } from "./baseline.ts";
import { generateReport, generateMetricsOnly } from "./report.ts";
import type { Scenario, AgentConfig, TrialResult, PromptTier, ClaudeMdMode } from "./types.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EVAL_ROOT = path.resolve(__dirname, "..");

const { values: args } = parseArgs({
  options: {
    scenario: { type: "string" },
    agent: { type: "string" },
    reps: { type: "string", default: "1" },
    tier: { type: "string" },
    "prompt-tier": { type: "string" },
    mode: { type: "string", default: "jig" },
    "claude-md": { type: "string", default: "shared" },
    "dry-run": { type: "boolean", default: false },
    "metrics-only": { type: "boolean", default: false },
  },
  strict: true,
});

const reps = parseInt(args.reps!, 10);
const mode = args.mode as "jig" | "baseline";
const promptTierFilter = args["prompt-tier"] as PromptTier | undefined;
const claudeMd = args["claude-md"] as ClaudeMdMode;
const VALID_CLAUDE_MD: ClaudeMdMode[] = ["shared", "empty", "none"];

// Validate --claude-md
if (!VALID_CLAUDE_MD.includes(claudeMd)) {
  console.error(`Invalid --claude-md "${claudeMd}". Valid: ${VALID_CLAUDE_MD.join(", ")}`);
  process.exit(1);
}

// Validate --prompt-tier
if (promptTierFilter && !VALID_PROMPT_TIERS.includes(promptTierFilter)) {
  console.error(`Invalid --prompt-tier "${promptTierFilter}". Valid: ${VALID_PROMPT_TIERS.join(", ")}`);
  process.exit(1);
}

// Block contradictory directed + baseline
if (mode === "baseline" && promptTierFilter === "directed") {
  console.error("Cannot run --prompt-tier directed with --mode baseline. Directed prompts reference skills explicitly.");
  process.exit(1);
}

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
  let totalTrials = 0;
  const byTier: Record<string, number> = {};
  const byPromptTier: Record<string, number> = {};

  for (const s of scenarios) {
    byTier[s.tier] = (byTier[s.tier] ?? 0) + 1;
    const tiers = getPromptTiersForScenario(s);
    for (const pt of tiers) {
      byPromptTier[pt] = (byPromptTier[pt] ?? 0) + 1;
    }
    totalTrials += tiers.length * agents.length * reps;
  }

  console.error(`Dry run: ${scenarios.length} scenarios x ${agents.length} agents x ${reps} reps = ${totalTrials} trials`);
  console.error("By difficulty tier:", JSON.stringify(byTier));
  console.error("By prompt tier:", JSON.stringify(byPromptTier));
  console.error(`Mode: ${mode}`);
  console.error(`CLAUDE.md: ${claudeMd}`);
  if (promptTierFilter) console.error(`Prompt tier filter: ${promptTierFilter}`);
  console.error(`Jig version: ${jigVersion}`);
  console.error("Scenarios:");
  for (const s of scenarios) {
    const tiers = getPromptTiersForScenario(s);
    console.error(`  - ${s.name} (${s.tier}) [${tiers.join(", ")}]`);
  }
  console.error("Agents:");
  for (const a of agents) {
    console.error(`  - ${a.name}`);
  }
  process.exit(0);
}

// Determine which prompt tiers to run for a scenario
function getPromptTiersForScenario(scenario: Scenario): PromptTier[] {
  let tiers = VALID_PROMPT_TIERS.filter(
    (t) => scenario.prompts[t] && scenario.prompts[t]!.trim() !== ""
  );

  if (promptTierFilter) {
    tiers = tiers.filter((t) => t === promptTierFilter);
  }

  // Skip directed in baseline mode
  if (mode === "baseline") {
    tiers = tiers.filter((t) => t !== "directed");
  }

  return tiers;
}

// Run trials
const resultsPath = path.join(EVAL_ROOT, "results", "results.jsonl");
let totalTrials = 0;
for (const s of scenarios) {
  totalTrials += getPromptTiersForScenario(s).length * agents.length * reps;
}
let trialNum = 0;

for (const scenario of scenarios) {
  const tiersToRun = getPromptTiersForScenario(scenario);
  if (tiersToRun.length === 0) continue;

  for (const agent of agents) {
    for (const promptTier of tiersToRun) {
      for (let rep = 1; rep <= reps; rep++) {
        trialNum++;
        let sandbox;
        try {
          sandbox = await createSandbox(scenario, claudeMd);
        } catch (err) {
          console.error(`[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} [${promptTier}] rep=${rep}  SANDBOX FAILED: ${err}`);
          continue;
        }

        try {
          // Assemble prompt from the selected tier
          const rawPrompt = scenario.prompts[promptTier]!;
          const prompt = mode === "baseline"
            ? transformPromptForBaseline(rawPrompt)
            : (scenario.context ? `${scenario.context}\n\n${rawPrompt}` : rawPrompt);

          // Invoke agent
          console.error(`  [debug] cwd=${sandbox.workDir}`);
          console.error(`  [debug] prompt_tier=${promptTier}`);
          console.error(`  [debug] cmd=${agent.command} ${agent.args.join(" ")} "${prompt.slice(0, 100)}..."`);
          const agentResult = await invokeAgent(agent, prompt, sandbox.workDir);

          // Score — timeout trials get zeroed
          let assertionResults;
          let negativeResults;
          let fileSc;
          let jigUsage;
          let efficiency;
          let trialScore;

          if (agentResult.timedOut) {
            assertionResults = scenario.assertions.map((a) => ({ ...a, passed: false }));
            negativeResults = { passed: true, results: [] };
            fileSc = 0;
            jigUsage = { jig_used: false, jig_correct: false, call_count: 0, invocations: [] };
            efficiency = { tool_calls: 0, tokens_used: 0, cost_usd: 0 };
            trialScore = { assertion_score: 0, file_score: 0, negative_score: 0, jig_used: false, jig_correct: false, total: 0 };
          } else {
            assertionResults = scoreAssertions(scenario, sandbox.workDir);
            negativeResults = scoreNegativeAssertions(scenario, sandbox.workDir);
            fileSc = aggregateFileScore(scenario, sandbox.workDir);
            const agentOutput = agentResult.stdout + "\n" + agentResult.stderr;
            jigUsage = scoreJigUsage(agentOutput, scenario);
            efficiency = scoreEfficiency(agentResult.stdout);
            trialScore = computeTrialScore(assertionResults, negativeResults, fileSc, jigUsage);
          }

          const result: TrialResult = {
            scenario: scenario.name,
            agent: agent.name,
            mode,
            prompt_tier: promptTier,
            claude_md: claudeMd,
            rep,
            tier: scenario.tier,
            category: scenario.category,
            timestamp: new Date().toISOString(),
            duration_ms: agentResult.durationMs,
            jig_version: sandbox.jigVersion || jigVersion,
            scores: trialScore,
            assertions: assertionResults,
            negative_assertions: negativeResults.results,
            jig_invocations: jigUsage.invocations,
            agent_exit_code: agentResult.exitCode,
            agent_tool_calls: efficiency.tool_calls,
            tokens_used: efficiency.tokens_used,
            cost_usd: efficiency.cost_usd,
            timeout: agentResult.timedOut,
            tags: scenario.tags ?? [],
          };

          writeTrialResult(result, resultsPath);

          // Debug: dump agent output and modified files when assertions fail
          const anyFailed = assertionResults.some((a) => !a.passed);
          if (anyFailed) {
            console.error(`  [debug] agent stderr (last 500):\n${agentResult.stderr.slice(-500)}`);
            console.error(`  [debug] agent stdout (last 500):\n${agentResult.stdout.slice(-500)}`);
            for (const f of scenario.expected_files_modified) {
              const fp = path.join(sandbox.workDir, f);
              if (fs.existsSync(fp)) {
                console.error(`  [debug] ${f}:\n${fs.readFileSync(fp, "utf-8")}`);
              } else {
                console.error(`  [debug] ${f}: NOT FOUND`);
              }
            }
          }

          const elapsed = (agentResult.durationMs / 1000).toFixed(1);
          console.error(
            `[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} [${promptTier}] rep=${rep}  score=${trialScore.total.toFixed(2)}  ${elapsed}s`
          );
        } catch (err) {
          console.error(`[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} [${promptTier}] rep=${rep}  ERROR: ${err}`);
        } finally {
          await sandbox.cleanup();
        }
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
