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
import { writeAgentArtifacts } from "./artifacts.ts";
import { writeTrialResult, readResults, ResultSchemaError, formatDiagnosticsSummary } from "./results.ts";
import { transformPromptForBaseline } from "./baseline.ts";
import { generateReport, generateMetricsOnly } from "./report.ts";
import type {
  Scenario,
  AgentConfig,
  TrialResult,
  PromptTier,
  ClaudeMdMode,
  SchemaPolicyMode,
  AgentArtifactPaths,
} from "./types.ts";

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
    "strip-skills": { type: "boolean", default: false },
    "dry-run": { type: "boolean", default: false },
    "metrics-only": { type: "boolean", default: false },
    "schema-mode": { type: "string", default: "strict" },
    "disable-jig-binary": { type: "boolean", default: false },
    "emit-jig-plan": { type: "boolean", default: false },
    "artifacts-dir": { type: "string", default: "results/artifacts" },
    "no-capture-artifacts": { type: "boolean", default: false },
    results: { type: "string" },
  },
  strict: true,
});

const reps = parseInt(args.reps!, 10);
const mode = args.mode as "jig" | "baseline";
const stripSkills = args["strip-skills"] ?? false;
const disableJigBinary = args["disable-jig-binary"] ?? false;
const emitJigPlan = args["emit-jig-plan"] ?? false;
const captureArtifacts = !(args["no-capture-artifacts"] ?? false);
const artifactsDirArg = args["artifacts-dir"];
const promptTierFilter = args["prompt-tier"] as PromptTier | undefined;
const claudeMd = args["claude-md"] as ClaudeMdMode;
const VALID_CLAUDE_MD: ClaudeMdMode[] = ["shared", "empty", "none"];
const schemaMode = args["schema-mode"] as SchemaPolicyMode;
const VALID_SCHEMA_MODES: SchemaPolicyMode[] = ["strict", "compat"];

// Validate --claude-md
if (!VALID_CLAUDE_MD.includes(claudeMd)) {
  console.error(`Invalid --claude-md "${claudeMd}". Valid: ${VALID_CLAUDE_MD.join(", ")}`);
  process.exit(1);
}

if (!VALID_SCHEMA_MODES.includes(schemaMode)) {
  console.error(`Invalid --schema-mode "${schemaMode}". Valid: ${VALID_SCHEMA_MODES.join(", ")}`);
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

const resultsPath = args.results
  ? path.resolve(process.cwd(), args.results)
  : path.join(EVAL_ROOT, "results", "results.jsonl");
const artifactsRoot = artifactsDirArg
  ? path.resolve(process.cwd(), artifactsDirArg)
  : path.join(EVAL_ROOT, "results", "artifacts");

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
  console.error(`Schema mode: ${schemaMode}`);
  console.error(`Results path: ${resultsPath}`);
  console.error(`Capture artifacts: ${captureArtifacts}`);
  if (captureArtifacts) console.error(`Artifacts dir: ${artifactsRoot}`);
  console.error(`CLAUDE.md: ${claudeMd}`);
  console.error(`Strip skills: ${stripSkills}`);
  console.error(`Disable jig binary: ${disableJigBinary}`);
  console.error(`Emit JIG_PLAN pre-tool note: ${emitJigPlan}`);
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

function addJigPlanDiagnosticInstruction(prompt: string): string {
  return `${prompt}

Diagnostic requirement for this run:
Before your first tool call, output exactly one line in this format:
JIG_PLAN: {"goal":"...","recipe":"...","vars":{"key":"value"},"sources":{"goal":"<where this came from>","recipe":"<where recipe name came from>","vars":{"key":"<where this var came from>"}},"command":"...","command_source":"<where command choice came from>"}
For each field in vars, sources.vars must include the origin (for example: "$ARGUMENTS", "directed prompt", "natural prompt", "SKILL.md", "jig list", or "inferred").
Keep it brief and factual, then continue with normal execution.`;
}

// Run trials

// Strict mode fail-fast before expensive trial execution.
if (schemaMode === "strict" && fs.existsSync(resultsPath)) {
  try {
    readResults(resultsPath, { schemaMode });
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
}

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
          sandbox = await createSandbox(scenario, claudeMd, stripSkills, disableJigBinary);
        } catch (err) {
          console.error(`[${trialNum}/${totalTrials}] ${scenario.name} x ${agent.name} [${promptTier}] rep=${rep}  SANDBOX FAILED: ${err}`);
          continue;
        }

        try {
          // Assemble prompt from the selected tier
          const rawPrompt = scenario.prompts[promptTier]!;
          let prompt = mode === "baseline"
            ? transformPromptForBaseline(rawPrompt)
            : (scenario.context ? `${scenario.context}\n\n${rawPrompt}` : rawPrompt);
          if (mode === "jig" && emitJigPlan) {
            prompt = addJigPlanDiagnosticInstruction(prompt);
          }

          // Invoke agent
          console.error(`  [debug] cwd=${sandbox.workDir}`);
          console.error(`  [debug] prompt_tier=${promptTier}`);
          if (disableJigBinary && sandbox.jigShimDir) {
            console.error(`  [debug] jig disabled via shim dir=${sandbox.jigShimDir}`);
          }
          console.error(`  [debug] cmd=${agent.command} ${agent.args.join(" ")} "${prompt.slice(0, 100)}..."`);
          const envOverrides = (disableJigBinary && sandbox.jigShimDir)
            ? { PATH: `${sandbox.jigShimDir}${path.delimiter}${process.env.PATH ?? ""}` }
            : undefined;
          const agentResult = await invokeAgent(agent, prompt, sandbox.workDir, envOverrides);
          let artifactPaths: AgentArtifactPaths | undefined;
          if (captureArtifacts) {
            try {
              artifactPaths = writeAgentArtifacts({
                artifactsRoot,
                scenario: scenario.name,
                agent: agent.name,
                promptTier,
                rep,
                mode,
                prompt,
                stdout: agentResult.stdout,
                stderr: agentResult.stderr,
                workDir: sandbox.workDir,
              });
              console.error(`  [debug] artifacts=${artifactPaths.dir}`);
            } catch (err) {
              console.error(`  [debug] artifact write failed: ${err}`);
            }
          }

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
            efficiency = { tool_calls: 0, input_tokens: 0, output_tokens: 0, cache_creation_input_tokens: 0, cache_read_input_tokens: 0, tokens_used: 0, cost_usd: 0 };
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
            input_tokens: efficiency.input_tokens,
            output_tokens: efficiency.output_tokens,
            cache_creation_input_tokens: efficiency.cache_creation_input_tokens,
            cache_read_input_tokens: efficiency.cache_read_input_tokens,
            tokens_used: efficiency.tokens_used,
            cost_usd: efficiency.cost_usd,
            timeout: agentResult.timedOut,
            skills_available: sandbox.skillsAvailable,
            tags: scenario.tags ?? [],
            agent_artifacts: artifactPaths,
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
let loadedResults;
try {
  loadedResults = readResults(resultsPath, { schemaMode });
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

if (loadedResults.results.length > 0) {
  if (args["metrics-only"]) {
    console.log(generateMetricsOnly(loadedResults.results));
  } else {
    console.error(generateReport(loadedResults.results));
    console.log(generateMetricsOnly(loadedResults.results));
  }
}
