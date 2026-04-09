import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { loadAllScenarios, validateScenario } from "../harness/scenarios.ts";
import { getAgentByName, invokeAgent, loadAgentConfigs } from "../harness/agents.ts";
import {
  computeTrialScore,
  scoreAssertions,
  scoreJigUsage,
  scoreNegativeAssertions,
} from "../harness/score.ts";
import { aggregateFileScore } from "../lib/diff.ts";
import { writeHeadToHeadArtifacts } from "./artifacts.ts";
import {
  buildPairResult,
  summarizePairs,
  writeHeadToHeadPairResult,
  writeHeadToHeadTrialResult,
} from "./results.ts";
import { createHeadToHeadSandbox } from "./sandbox.ts";
import { extractHeadToHeadTelemetry } from "./telemetry.ts";
import type {
  HeadToHeadArmConfig,
  HeadToHeadPairResult,
  HeadToHeadPromptSource,
  HeadToHeadTrialResult,
} from "./types.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EVAL_ROOT = path.resolve(__dirname, "..");
const DEFAULT_THINKING_PREFIX = "HEAD2HEAD_THINKING:";
const VALID_PROMPT_SOURCES: HeadToHeadPromptSource[] = [
  "natural",
  "directed",
  "ambient",
  "legacy_prompt",
  "custom",
];

const { values: args } = parseArgs({
  options: {
    scenario: { type: "string" },
    agent: { type: "string", default: "claude-code" },
    reps: { type: "string", default: "1" },
    "control-profile": { type: "string" },
    "jig-profile": { type: "string" },
    "control-label": { type: "string", default: "control" },
    "jig-label": { type: "string", default: "jig" },
    "prompt-source": { type: "string", default: "natural" },
    "prompt-file": { type: "string" },
    "prompt-text": { type: "string" },
    "thinking-mode": { type: "boolean", default: false },
    "thinking-prefix": { type: "string", default: DEFAULT_THINKING_PREFIX },
    "preserve-codebase-claude": { type: "boolean", default: false },
    "dry-run": { type: "boolean", default: false },
    "no-capture-artifacts": { type: "boolean", default: false },
    "artifacts-dir": { type: "string", default: "results/head2head-artifacts" },
    results: { type: "string", default: "results/head2head-results.jsonl" },
    pairs: { type: "string", default: "results/head2head-pairs.jsonl" },
  },
  strict: true,
});

const reps = Number.parseInt(args.reps ?? "1", 10);
if (!Number.isFinite(reps) || reps < 1) {
  console.error(`Invalid --reps "${args.reps}". Must be >= 1.`);
  process.exit(1);
}

const promptSource = args["prompt-source"] as HeadToHeadPromptSource;
if (!VALID_PROMPT_SOURCES.includes(promptSource)) {
  console.error(`Invalid --prompt-source "${promptSource}". Valid: ${VALID_PROMPT_SOURCES.join(", ")}`);
  process.exit(1);
}

const controlProfile = args["control-profile"];
const jigProfile = args["jig-profile"];
if (!controlProfile || !jigProfile) {
  console.error("Both --control-profile and --jig-profile are required.");
  process.exit(1);
}

const customPromptText = resolveCustomPrompt(args["prompt-file"], args["prompt-text"]);
if (promptSource === "custom" && !customPromptText) {
  console.error("--prompt-source custom requires --prompt-file or --prompt-text.");
  process.exit(1);
}

const scenariosDir = path.join(EVAL_ROOT, "scenarios");
let scenarios = loadAllScenarios(scenariosDir);
if (scenarios.length === 0) {
  console.error(`No scenarios found under ${scenariosDir}.`);
  process.exit(1);
}

if (args.scenario) {
  const wantedNames = args.scenario
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
  const wantedSet = new Set(wantedNames);
  scenarios = scenarios.filter((scenario) => wantedSet.has(scenario.name));
  if (scenarios.length === 0) {
    console.error(`No scenarios matched --scenario "${args.scenario}".`);
    process.exit(1);
  }
}

const validationErrors: string[] = [];
for (const scenario of scenarios) {
  for (const err of validateScenario(scenario)) {
    validationErrors.push(`${scenario.name}: ${err.field} — ${err.message}`);
  }
}

if (validationErrors.length > 0) {
  console.error("Scenario validation failed:");
  for (const line of validationErrors) {
    console.error(`  - ${line}`);
  }
  process.exit(1);
}

const allAgents = loadAgentConfigs(path.join(EVAL_ROOT, "agents.yaml"));
const agent = getAgentByName(allAgents, args.agent!);

const arms: [HeadToHeadArmConfig, HeadToHeadArmConfig] = [
  {
    arm: "control",
    label: args["control-label"] ?? "control",
    profilePath: controlProfile,
  },
  {
    arm: "jig",
    label: args["jig-label"] ?? "jig",
    profilePath: jigProfile,
  },
];

const thinkingMode = args["thinking-mode"] ?? false;
const thinkingPrefix = args["thinking-prefix"] ?? DEFAULT_THINKING_PREFIX;
const cleanSlate = !(args["preserve-codebase-claude"] ?? false);
const captureArtifacts = !(args["no-capture-artifacts"] ?? false);
const resultsPath = path.resolve(process.cwd(), args.results!);
const pairsPath = path.resolve(process.cwd(), args.pairs!);
const artifactsRoot = path.resolve(process.cwd(), args["artifacts-dir"]!);

const runId = makeRunId();
const totalTrials = scenarios.length * reps * arms.length;

if (args["dry-run"]) {
  console.error(`Dry run only (no agent execution).`);
  console.error(`Run ID: ${runId}`);
  console.error(`Trials: ${scenarios.length} scenarios x ${reps} reps x ${arms.length} arms = ${totalTrials}`);
  console.error(`Agent: ${agent.name}`);
  console.error(`Prompt source: ${promptSource}`);
  console.error(`Thinking mode: ${thinkingMode}`);
  console.error(`Clean slate: ${cleanSlate}`);
  console.error(`Control profile: ${path.resolve(controlProfile)}`);
  console.error(`Jig profile: ${path.resolve(jigProfile)}`);
  console.error(`Results path: ${resultsPath}`);
  console.error(`Pairs path: ${pairsPath}`);
  console.error(`Artifacts capture: ${captureArtifacts}`);
  if (captureArtifacts) {
    console.error(`Artifacts dir: ${artifactsRoot}`);
  }
  console.error("Scenarios:");
  for (const scenario of scenarios) {
    console.error(`  - ${scenario.name}`);
  }
  process.exit(0);
}

const pairMap = new Map<string, Partial<Record<"control" | "jig", HeadToHeadTrialResult>>>();
const pairResults: HeadToHeadPairResult[] = [];
let trialNum = 0;

for (const scenario of scenarios) {
  const scenarioPrompt = resolveScenarioPrompt(scenario, promptSource, customPromptText);

  for (let rep = 1; rep <= reps; rep++) {
    for (const arm of arms) {
      trialNum++;
      let sandbox;

      try {
        sandbox = await createHeadToHeadSandbox(scenario, arm, { cleanSlate });
      } catch (err) {
        console.error(
          `[${trialNum}/${totalTrials}] ${scenario.name} [${arm.arm}] rep=${rep} sandbox failed: ${(err as Error).message}`
        );
        continue;
      }

      try {
        const prompt = buildPrompt(scenario.context, scenarioPrompt, thinkingMode, thinkingPrefix);
        const agentResult = await invokeAgent(agent, prompt, sandbox.workDir);

        const artifactPaths = captureArtifacts
          ? writeHeadToHeadArtifacts({
            artifactsRoot,
            scenario: scenario.name,
            agent: agent.name,
            arm: arm.arm,
            rep,
            prompt,
            stdout: agentResult.stdout,
            stderr: agentResult.stderr,
            workDir: sandbox.workDir,
          })
          : undefined;

        const combinedAgentOutput = `${agentResult.stdout}\n${agentResult.stderr}`;
        const jigUsage = scoreJigUsage(combinedAgentOutput, scenario);

        let assertionResults;
        let negativeResults;
        let fileScoreValue;
        let trialScore;
        if (agentResult.timedOut) {
          assertionResults = scenario.assertions.map((assertion) => ({ ...assertion, passed: false }));
          negativeResults = { passed: true, results: [] };
          fileScoreValue = 0;
          trialScore = {
            assertion_score: 0,
            file_score: 0,
            negative_score: 0,
            jig_used: false,
            jig_correct: false,
            total: 0,
          };
        } else {
          assertionResults = scoreAssertions(scenario, sandbox.workDir);
          negativeResults = scoreNegativeAssertions(scenario, sandbox.workDir);
          fileScoreValue = aggregateFileScore(scenario, sandbox.workDir);
          trialScore = computeTrialScore(assertionResults, negativeResults, fileScoreValue, jigUsage);
        }

        const telemetryExtraction = extractHeadToHeadTelemetry(agentResult, thinkingPrefix);
        const result: HeadToHeadTrialResult = {
          schema_version: "head2head_v1",
          timestamp: new Date().toISOString(),
          run_id: runId,
          scenario: scenario.name,
          rep,
          agent: agent.name,
          arm: arm.arm,
          arm_label: arm.label,
          profile_path: path.resolve(arm.profilePath),
          prompt_source: promptSource,
          thinking_mode: thinkingMode,
          thinking_text: telemetryExtraction.thinkingText,
          tier: scenario.tier,
          category: scenario.category,
          tags: scenario.tags ?? [],
          duration_ms: agentResult.durationMs,
          jig_version: sandbox.jigVersion,
          installed_skills: sandbox.installedSkills,
          has_claude_md: sandbox.hasClaudeMd,
          scores: trialScore,
          file_score: fileScoreValue,
          assertions: assertionResults,
          negative_assertions: negativeResults.results,
          jig_invocations: jigUsage.invocations,
          agent_exit_code: agentResult.exitCode,
          timeout: agentResult.timedOut,
          telemetry: telemetryExtraction.telemetry,
          agent_artifacts: artifactPaths,
        };

        writeHeadToHeadTrialResult(result, resultsPath);

        const pairKey = `${scenario.name}|${rep}|${agent.name}`;
        const existingPair = pairMap.get(pairKey) ?? {};
        existingPair[arm.arm] = result;
        pairMap.set(pairKey, existingPair);

        if (existingPair.control && existingPair.jig) {
          const pair = buildPairResult(
            runId,
            scenario.name,
            rep,
            agent.name,
            existingPair.control,
            existingPair.jig
          );
          pairResults.push(pair);
          writeHeadToHeadPairResult(pair, pairsPath);
        }

        const elapsedSec = (agentResult.durationMs / 1000).toFixed(1);
        const tokens = result.telemetry.tokens_used != null
          ? result.telemetry.tokens_used.toLocaleString()
          : "n/a";
        const cost = result.telemetry.cost_usd != null
          ? result.telemetry.cost_usd.toFixed(4)
          : "n/a";
        console.error(
          `[${trialNum}/${totalTrials}] ${scenario.name} [${arm.arm}] rep=${rep} score=${result.scores.total.toFixed(2)} tokens=${tokens} cost=${cost} ${elapsedSec}s`
        );
      } catch (err) {
        console.error(
          `[${trialNum}/${totalTrials}] ${scenario.name} [${arm.arm}] rep=${rep} failed: ${(err as Error).message}`
        );
      } finally {
        await sandbox.cleanup();
      }
    }
  }
}

console.error("");
console.error(summarizePairs(pairResults));
console.error("");
console.error(`Trial results: ${resultsPath}`);
console.error(`Pair results: ${pairsPath}`);

function resolveCustomPrompt(promptFile: string | undefined, promptText: string | undefined): string | undefined {
  if (promptText && promptText.trim().length > 0) {
    return promptText.trim();
  }
  if (promptFile && promptFile.trim().length > 0) {
    return fs.readFileSync(path.resolve(process.cwd(), promptFile), "utf-8").trim();
  }
  return undefined;
}

function resolveScenarioPrompt(
  scenario: { prompts: Partial<Record<"natural" | "directed" | "ambient", string>>; prompt: string; name: string },
  promptSource: HeadToHeadPromptSource,
  customPromptText?: string
): string {
  if (promptSource === "custom") {
    if (!customPromptText) {
      throw new Error("custom prompt requested without prompt text");
    }
    return customPromptText;
  }

  if (promptSource === "legacy_prompt") {
    if (!scenario.prompt || scenario.prompt.trim().length === 0) {
      throw new Error(`Scenario ${scenario.name} has no legacy prompt field.`);
    }
    return scenario.prompt;
  }

  const prompt = scenario.prompts[promptSource];
  if (!prompt || prompt.trim().length === 0) {
    throw new Error(`Scenario ${scenario.name} has no ${promptSource} prompt.`);
  }
  return prompt;
}

function buildPrompt(context: string | undefined, prompt: string, thinkingMode: boolean, thinkingPrefix: string): string {
  let out = context && context.trim().length > 0 ? `${context.trim()}\n\n${prompt.trim()}` : prompt.trim();

  if (thinkingMode) {
    out = `${out}\n\nDiagnostic requirement for this run:\nBefore your first tool call, output exactly one line beginning with:\n${thinkingPrefix} <what you plan to do first and why>\nThen continue with normal execution.`;
  }

  return out;
}

function makeRunId(): string {
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const rand = Math.random().toString(36).slice(2, 8);
  return `${ts}__${rand}`;
}
