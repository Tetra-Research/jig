import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { loadScenario, loadAllScenarios, validateScenario } from "./scenarios.ts";
import { scoreAssertions, scoreNegativeAssertions, scoreJigUsage, computeTrialScore } from "./score.ts";
import { normalizeFile } from "../lib/normalize.ts";
import { fileScore, aggregateFileScore } from "../lib/diff.ts";
import { writeTrialResult, readResults } from "./results.ts";
import { createSandbox } from "../lib/sandbox.ts";
import { loadAgentConfigs, getAgentByName, invokeAgent } from "./agents.ts";
import { transformPromptForBaseline } from "./baseline.ts";
import { generateReport, generateMetricsOnly, aggregate } from "./report.ts";
import type { TrialResult, Scenario } from "./types.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.join(__dirname, "test-fixtures");

let passed = 0;
let failed = 0;

async function test(name: string, fn: () => void | Promise<void>) {
  try {
    await fn();
    passed++;
    console.log(`  PASS  ${name}`);
  } catch (err) {
    failed++;
    console.log(`  FAIL  ${name}`);
    console.log(`        ${(err as Error).message}`);
  }
}

// ── Scenario loading tests ──

console.log("\n--- scenarios.ts ---");

await test("loadScenario: loads valid scenario with all fields", () => {
  const s = loadScenario(path.join(FIXTURES, "valid-scenario"));
  assert.strictEqual(s.name, "test-scenario");
  assert.strictEqual(s.description, "A minimal valid scenario for testing");
  assert.strictEqual(s.tier, "easy");
  assert.strictEqual(s.category, "test");
  assert.ok(s.prompt.includes("greeting function"));
  assert.ok(s.context?.includes("Python project"));
  assert.deepStrictEqual(s.expected_files_modified, ["hello.py"]);
  assert.strictEqual(s.assertions.length, 2);
  assert.strictEqual(s.assertions[0].file, "hello.py");
  assert.strictEqual(s.assertions[0].contains, "def greet");
  assert.strictEqual(s.assertions[0].weight, 1.0);
  assert.strictEqual(s.assertions[1].weight, 0.5);
  assert.strictEqual(s.negative_assertions?.length, 1);
  assert.deepStrictEqual(s.tags, ["test", "easy"]);
  assert.strictEqual(s.estimated_jig_commands, 1);
  assert.strictEqual(s.max_jig_commands, 2);
  assert.ok(path.isAbsolute(s.scenarioDir));
});

await test("loadScenario: defaults weight to 1.0 when not specified", () => {
  const s = loadScenario(path.join(FIXTURES, "invalid-scenarios", "missing-name"));
  assert.strictEqual(s.assertions[0].weight, 1.0);
});

await test("loadAllScenarios: loads in deterministic lexicographic order", () => {
  const scenarios = loadAllScenarios(path.join(FIXTURES, "invalid-scenarios"));
  // Directories are sorted lexicographically: bad-tier, missing-name, no-codebase-dir
  const dirNames = scenarios.map((s) => path.basename(s.scenarioDir));
  const sorted = [...dirNames].sort();
  assert.deepStrictEqual(dirNames, sorted);
});

await test("loadAllScenarios: returns empty for nonexistent directory", () => {
  const scenarios = loadAllScenarios("/nonexistent/path");
  assert.deepStrictEqual(scenarios, []);
});

await test("validateScenario: valid scenario produces no errors", () => {
  const s = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const errors = validateScenario(s);
  assert.strictEqual(errors.length, 0);
});

await test("validateScenario: missing name produces error", () => {
  const s = loadScenario(path.join(FIXTURES, "invalid-scenarios", "missing-name"));
  const errors = validateScenario(s);
  const nameError = errors.find((e) => e.field === "name");
  assert.ok(nameError, "Should report missing name");
});

await test("validateScenario: invalid tier produces error with valid tiers listed", () => {
  const s = loadScenario(path.join(FIXTURES, "invalid-scenarios", "bad-tier"));
  const errors = validateScenario(s);
  const tierError = errors.find((e) => e.field === "tier");
  assert.ok(tierError, "Should report invalid tier");
  assert.ok(tierError!.message.includes("easy"), "Should list valid tiers");
});

await test("validateScenario: missing codebase dir produces error", () => {
  const s = loadScenario(path.join(FIXTURES, "invalid-scenarios", "no-codebase-dir"));
  const errors = validateScenario(s);
  const cbError = errors.find((e) => e.field === "codebase");
  assert.ok(cbError, "Should report missing codebase/");
});

await test("validateScenario: reports ALL errors, not just first", () => {
  // no-codebase-dir is also missing expected/ dir
  const s = loadScenario(path.join(FIXTURES, "invalid-scenarios", "no-codebase-dir"));
  const errors = validateScenario(s);
  // Should have at least codebase + expected errors
  assert.ok(errors.length >= 2, `Expected at least 2 errors, got ${errors.length}`);
});

// ── Normalize tests ──

console.log("\n--- normalize.ts ---");

await test("normalizeFile: strips trailing whitespace", () => {
  const result = normalizeFile("hello   \nworld  \n");
  assert.strictEqual(result, "hello\nworld\n");
});

await test("normalizeFile: normalizes CRLF to LF", () => {
  const result = normalizeFile("hello\r\nworld\r\n");
  assert.strictEqual(result, "hello\nworld\n");
});

await test("normalizeFile: removes trailing blank lines", () => {
  const result = normalizeFile("hello\nworld\n\n\n\n");
  assert.strictEqual(result, "hello\nworld\n");
});

// ── Diff tests ──

console.log("\n--- diff.ts ---");

await test("fileScore: identical files return 1.0", () => {
  const score = fileScore("hello\nworld\n", "hello\nworld\n");
  assert.strictEqual(score, 1.0);
});

await test("fileScore: identical after normalization returns 1.0", () => {
  const score = fileScore("hello  \r\nworld\n\n", "hello\nworld\n");
  assert.strictEqual(score, 1.0);
});

await test("fileScore: different files return Jaccard similarity", () => {
  const score = fileScore("line1\nline2\nline3\n", "line1\nline2\nline4\n");
  // intersection = {line1, line2} = 2, union = {line1, line2, line3, line4} = 4
  assert.strictEqual(score, 0.5);
});

await test("fileScore: completely different files return 0", () => {
  const score = fileScore("aaa\n", "bbb\n");
  assert.strictEqual(score, 0);
});

await test("fileScore: both empty returns 1.0", () => {
  const score = fileScore("", "");
  assert.strictEqual(score, 1.0);
});

await test("aggregateFileScore: missing file scores 0.0", () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  // workDir has no hello.py, so it should score 0
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-test-"));
  try {
    const score = aggregateFileScore(scenario, tmpDir);
    assert.strictEqual(score, 0.0);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

// ── Scoring tests ──

console.log("\n--- score.ts ---");

await test("scoreAssertions: passes when contains string is found", () => {
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  // Use expected/ dir as the workDir (it has the "after" state)
  const workDir = path.join(FIXTURES, "scoring", "expected");
  const results = scoreAssertions(scenario, workDir);
  const loyaltyResult = results.find((r) => r.contains === "loyalty_tier");
  assert.ok(loyaltyResult?.passed, "loyalty_tier should be found");
});

await test("scoreAssertions: fails when contains string is not found", () => {
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  // Use codebase/ as workDir (the "before" state — no loyalty_tier)
  const workDir = path.join(FIXTURES, "scoring", "codebase");
  const results = scoreAssertions(scenario, workDir);
  const loyaltyResult = results.find((r) => r.contains === "loyalty_tier");
  assert.ok(!loyaltyResult?.passed, "loyalty_tier should NOT be found in before state");
});

await test("scoreAssertions: scope narrows search to class body", () => {
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  // The assertion has scope: "class Reservation" and checks for "loyalty_tier"
  // In expected/models.py, loyalty_tier is inside class Reservation
  const workDir = path.join(FIXTURES, "scoring", "expected");
  const results = scoreAssertions(scenario, workDir);
  const scopedResult = results.find((r) => r.scope === "class Reservation");
  assert.ok(scopedResult?.passed, "Should find loyalty_tier within class Reservation scope");
});

await test("scoreAssertions: missing file marks assertion as failed", () => {
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-test-"));
  try {
    const results = scoreAssertions(scenario, tmpDir);
    assert.ok(results.every((r) => !r.passed), "All assertions should fail for missing files");
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

await test("scoreAssertions: weights are preserved", () => {
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const workDir = path.join(FIXTURES, "scoring", "expected");
  const results = scoreAssertions(scenario, workDir);
  const weighted = results.find((r) => r.weight === 2.0);
  assert.ok(weighted, "Should preserve weight=2.0");
});

await test("scoreNegativeAssertions: passes when not_contains is absent", () => {
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const workDir = path.join(FIXTURES, "scoring", "expected");
  const { passed: allPassed, results } = scoreNegativeAssertions(scenario, workDir);
  assert.ok(allPassed, "All negative assertions should pass");
  assert.ok(results.length > 0, "Should have results");
});

await test("scoreNegativeAssertions: fails when not_contains is found", () => {
  // Create a temp dir with a file containing the forbidden pattern
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-test-"));
  try {
    fs.writeFileSync(path.join(tmpDir, "models.py"), "SyntaxError here\n");
    const scenario = loadScenario(path.join(FIXTURES, "scoring"));
    const { passed: allPassed } = scoreNegativeAssertions(scenario, tmpDir);
    assert.ok(!allPassed, "Should fail when forbidden pattern is found");
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

await test("scoreNegativeAssertions: any_file checks all files", () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-test-"));
  try {
    fs.writeFileSync(path.join(tmpDir, "models.py"), "clean file\n");
    fs.writeFileSync(path.join(tmpDir, "other.py"), "FORBIDDEN_PATTERN\n");
    const scenario = loadScenario(path.join(FIXTURES, "scoring"));
    const { passed: allPassed, results } = scoreNegativeAssertions(scenario, tmpDir);
    const anyFileResult = results.find((r) => r.any_file);
    assert.ok(!anyFileResult?.passed, "any_file check should fail when pattern found in any file");
    assert.ok(!allPassed);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

await test("scoreJigUsage: detects jig run invocations", () => {
  const output = fs.readFileSync(path.join(FIXTURES, "mock-agent-output.txt"), "utf-8");
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const usage = scoreJigUsage(output, scenario);
  assert.ok(usage.jig_used, "Should detect jig usage");
  assert.strictEqual(usage.call_count, 2); // jig run + jig render
  assert.ok(usage.invocations.length === 2);
});

await test("scoreJigUsage: validates --vars JSON", () => {
  const output = `jig run recipe.yaml --vars '{"valid": true}'`;
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const usage = scoreJigUsage(output, scenario);
  assert.ok(usage.jig_correct, "Valid JSON vars should be correct");
});

await test("scoreJigUsage: invalid --vars JSON marks incorrect", () => {
  const output = `jig run recipe.yaml --vars '{invalid json}'`;
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const usage = scoreJigUsage(output, scenario);
  assert.ok(!usage.jig_correct, "Invalid JSON vars should be incorrect");
});

await test("scoreJigUsage: no jig usage returns jig_used=false", () => {
  const output = "Just editing files manually...";
  const scenario = loadScenario(path.join(FIXTURES, "scoring"));
  const usage = scoreJigUsage(output, scenario);
  assert.ok(!usage.jig_used);
  assert.strictEqual(usage.call_count, 0);
});

await test("computeTrialScore: correct weighted calculation", () => {
  const assertionResults = [
    { file: "a.py", contains: "x", passed: true, weight: 2.0 },
    { file: "b.py", contains: "y", passed: false, weight: 1.0 },
  ];
  const score = computeTrialScore(assertionResults, { passed: true }, 0.8, { jig_used: true, jig_correct: true });
  // assertion_score = 2.0 / 3.0 ≈ 0.667
  assert.ok(Math.abs(score.assertion_score - 2 / 3) < 0.001);
  assert.strictEqual(score.negative_score, 1.0);
  assert.ok(Math.abs(score.total - 2 / 3) < 0.001);
  assert.strictEqual(score.file_score, 0.8);
  assert.ok(score.jig_used);
  assert.ok(score.jig_correct);
});

await test("computeTrialScore: negative failure zeros total", () => {
  const assertionResults = [{ file: "a.py", contains: "x", passed: true, weight: 1.0 }];
  const score = computeTrialScore(assertionResults, { passed: false }, 1.0, { jig_used: true, jig_correct: true });
  assert.strictEqual(score.assertion_score, 1.0);
  assert.strictEqual(score.negative_score, 0.0);
  assert.strictEqual(score.total, 0.0);
});

// ── Results tests ──

console.log("\n--- results.ts ---");

await test("writeTrialResult + readResults: roundtrip preserves data", () => {
  const tmpFile = path.join(os.tmpdir(), `jig-test-results-${Date.now()}.jsonl`);
  try {
    const result: TrialResult = {
      scenario: "test",
      agent: "claude-code",
      mode: "jig",
      prompt_tier: "natural",
      claude_md: "shared",
      rep: 1,
      tier: "easy",
      category: "test",
      timestamp: new Date().toISOString(),
      duration_ms: 1234,
      jig_version: "jig 0.1.0",
      scores: { assertion_score: 0.75, file_score: 0.9, negative_score: 1.0, jig_used: true, jig_correct: true, total: 0.75 },
      assertions: [{ file: "a.py", contains: "x", passed: true, weight: 1.0 }],
      negative_assertions: [],
      jig_invocations: [{ command: "jig run recipe.yaml" }],
      agent_exit_code: 0,
      agent_tool_calls: 5,
      input_tokens: 8000,
      output_tokens: 2000,
      cache_creation_input_tokens: 0,
      cache_read_input_tokens: 0,
      tokens_used: 10000,
      cost_usd: 0.05,
      timeout: false,
      skills_available: true,
      tags: ["test"],
    };
    writeTrialResult(result, tmpFile);
    const read = readResults(tmpFile);
    assert.strictEqual(read.length, 1);
    assert.strictEqual(read[0].scenario, "test");
    assert.strictEqual(read[0].scores.assertion_score, 0.75);
    assert.deepStrictEqual(read[0].tags, ["test"]);
    assert.strictEqual(read[0].skills_available, true);
  } finally {
    try { fs.unlinkSync(tmpFile); } catch {}
  }
});

await test("writeTrialResult: appends without overwriting", () => {
  const tmpFile = path.join(os.tmpdir(), `jig-test-results-${Date.now()}.jsonl`);
  try {
    const base: TrialResult = {
      scenario: "s1", agent: "a1", mode: "jig", prompt_tier: "natural", claude_md: "shared", rep: 1, tier: "easy", category: "test",
      timestamp: new Date().toISOString(), duration_ms: 100, jig_version: "0.1.0",
      scores: { assertion_score: 1, file_score: 1, negative_score: 1, jig_used: true, jig_correct: true, total: 1 },
      assertions: [], negative_assertions: [], jig_invocations: [],
      agent_exit_code: 0, agent_tool_calls: 0, input_tokens: 0, output_tokens: 0, cache_creation_input_tokens: 0, cache_read_input_tokens: 0, tokens_used: 0, cost_usd: 0, timeout: false, skills_available: true, tags: [],
    };
    writeTrialResult({ ...base, scenario: "first" }, tmpFile);
    writeTrialResult({ ...base, scenario: "second" }, tmpFile);
    const read = readResults(tmpFile);
    assert.strictEqual(read.length, 2);
    assert.strictEqual(read[0].scenario, "first");
    assert.strictEqual(read[1].scenario, "second");
  } finally {
    try { fs.unlinkSync(tmpFile); } catch {}
  }
});

await test("readResults: returns empty array for nonexistent file", () => {
  const read = readResults("/nonexistent/results.jsonl");
  assert.deepStrictEqual(read, []);
});

// ── Sandbox tests ──

console.log("\n--- sandbox.ts ---");

await test("createSandbox: creates temp dir with codebase contents", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario);
  try {
    assert.ok(fs.existsSync(sandbox.workDir));
    assert.ok(fs.existsSync(path.join(sandbox.workDir, "main.py")));
    assert.ok(fs.readFileSync(path.join(sandbox.workDir, "main.py"), "utf-8").includes("hello"));
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: initializes git repo", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario);
  try {
    assert.ok(fs.existsSync(path.join(sandbox.workDir, ".git")));
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: captures jig version", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario);
  try {
    assert.ok(sandbox.jigVersion.length > 0, "jig version should be non-empty");
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: cleanup removes temp dir", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario);
  const dir = sandbox.workDir;
  await sandbox.cleanup();
  assert.ok(!fs.existsSync(dir), "Temp dir should be removed after cleanup");
});

await test("createSandbox: unique temp dirs for same scenario", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const s1 = await createSandbox(scenario);
  const s2 = await createSandbox(scenario);
  try {
    assert.notStrictEqual(s1.workDir, s2.workDir, "Each sandbox should have a unique dir");
  } finally {
    await s1.cleanup();
    await s2.cleanup();
  }
});

await test("createSandbox: claude-md=shared copies shared CLAUDE.md", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario, "shared");
  try {
    const claudeMdPath = path.join(sandbox.workDir, "CLAUDE.md");
    assert.ok(fs.existsSync(claudeMdPath), "CLAUDE.md should exist");
    const content = fs.readFileSync(claudeMdPath, "utf-8");
    assert.ok(content.includes("skill"), "Should contain shared CLAUDE.md content");
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: claude-md=empty writes minimal CLAUDE.md", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario, "empty");
  try {
    const claudeMdPath = path.join(sandbox.workDir, "CLAUDE.md");
    assert.ok(fs.existsSync(claudeMdPath), "CLAUDE.md should exist");
    const content = fs.readFileSync(claudeMdPath, "utf-8");
    assert.ok(!content.includes("skill"), "Should NOT contain shared content");
    assert.ok(content.includes("CLAUDE.md"), "Should have minimal content");
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: claude-md=none removes CLAUDE.md", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario, "none");
  try {
    const claudeMdPath = path.join(sandbox.workDir, "CLAUDE.md");
    assert.ok(!fs.existsSync(claudeMdPath), "CLAUDE.md should NOT exist");
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: strip-skills removes .claude/skills/", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario, "shared", true);
  try {
    const skillsDir = path.join(sandbox.workDir, ".claude", "skills");
    assert.ok(!fs.existsSync(skillsDir), ".claude/skills/ should be removed");
    assert.strictEqual(sandbox.skillsAvailable, false);
  } finally {
    await sandbox.cleanup();
  }
});

await test("createSandbox: strip-skills=false preserves .claude/skills/", async () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const sandbox = await createSandbox(scenario, "shared", false);
  try {
    const skillsDir = path.join(sandbox.workDir, ".claude", "skills");
    assert.ok(fs.existsSync(skillsDir), ".claude/skills/ should exist");
    assert.strictEqual(sandbox.skillsAvailable, true);
  } finally {
    await sandbox.cleanup();
  }
});

// ── Agent config tests (Phase 2.1) ──

console.log("\n--- agents.ts ---");

await test("loadAgentConfigs: loads valid agent configs", () => {
  const agentsPath = path.join(FIXTURES, "..", "..", "agents.yaml");
  const configs = loadAgentConfigs(agentsPath);
  assert.ok(configs.length >= 2, "Should load at least 2 agents");
  assert.strictEqual(configs[0].name, "claude-code");
  assert.strictEqual(configs[1].name, "claude-code-sonnet");
  assert.ok(configs[0].timeout_ms > 0);
  assert.ok(Array.isArray(configs[0].args));
});

await test("getAgentByName: returns agent when found", () => {
  const agentsPath = path.join(FIXTURES, "..", "..", "agents.yaml");
  const configs = loadAgentConfigs(agentsPath);
  const agent = getAgentByName(configs, "claude-code");
  assert.strictEqual(agent.name, "claude-code");
});

await test("getAgentByName: throws with available names for unknown agent", () => {
  const agentsPath = path.join(FIXTURES, "..", "..", "agents.yaml");
  const configs = loadAgentConfigs(agentsPath);
  assert.throws(
    () => getAgentByName(configs, "nonexistent"),
    (err: Error) => err.message.includes("claude-code") && err.message.includes("nonexistent")
  );
});

await test("invokeAgent: captures stdout from mock agent", async () => {
  const mockAgentPath = path.join(FIXTURES, "mock-agent.sh");
  const agent = { name: "mock", command: "bash", args: [mockAgentPath], timeout_ms: 5000 };
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-agent-test-"));
  try {
    const result = await invokeAgent(agent, "test prompt", tmpDir);
    assert.strictEqual(result.agent, "mock");
    assert.strictEqual(result.exitCode, 0);
    assert.ok(result.stdout.includes("Mock agent starting"));
    assert.ok(result.stdout.includes("jig run"));
    assert.ok(!result.timedOut);
    assert.ok(result.durationMs > 0);
    // Verify the mock agent wrote the file
    assert.ok(fs.existsSync(path.join(tmpDir, "hello.py")));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

await test("invokeAgent: timeout kills process", async () => {
  const mockTimeoutPath = path.join(FIXTURES, "mock-timeout-agent.sh");
  const agent = { name: "slow", command: "bash", args: [mockTimeoutPath], timeout_ms: 500 };
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "jig-timeout-test-"));
  try {
    const result = await invokeAgent(agent, "test", tmpDir);
    assert.ok(result.timedOut, "Should report timeout");
    assert.strictEqual(result.exitCode, -1);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

// ── Baseline tests (Phase 2.2) ──

console.log("\n--- baseline.ts ---");

await test("transformPromptForBaseline: replaces context with baseline instructions", () => {
  const scenario = loadScenario(path.join(FIXTURES, "valid-scenario"));
  const prompt = transformPromptForBaseline(scenario.prompt);
  assert.ok(prompt.includes("Do not use jig"), "Should include baseline instruction");
  assert.ok(prompt.includes("greeting function"), "Should preserve the core prompt");
});

await test("transformPromptForBaseline: strips jig references", () => {
  const prompt = "Add a field.\nUse jig run recipes/add-field to automate.\nPass --vars to provide values.";
  const result = transformPromptForBaseline(prompt);
  assert.ok(!result.includes("jig run"), "Should strip jig run references");
  assert.ok(!result.includes("--vars"), "Should strip --vars references");
  assert.ok(result.includes("Add a field"), "Should preserve non-jig content");
});

await test("transformPromptForBaseline: no jig run/workflow/library remain", () => {
  const prompt = "Do things.\njig run something.\njig workflow deploy.\njig library add x.";
  const result = transformPromptForBaseline(prompt);
  assert.ok(!result.includes("jig run"), "No jig run");
  assert.ok(!result.includes("jig workflow"), "No jig workflow");
  assert.ok(!result.includes("jig library"), "No jig library");
});

// ── Report tests (Phase 4.1) ──

console.log("\n--- report.ts ---");

function makeResult(overrides: Partial<TrialResult> = {}): TrialResult {
  return {
    scenario: "test-scenario",
    agent: "claude-code",
    mode: "jig",
    prompt_tier: "natural",
    claude_md: "shared",
    rep: 1,
    tier: "easy",
    category: "test",
    timestamp: new Date().toISOString(),
    duration_ms: 5000,
    jig_version: "jig 0.1.0",
    scores: { assertion_score: 0.8, file_score: 0.9, negative_score: 1.0, jig_used: true, jig_correct: true, total: 0.8 },
    assertions: [{ file: "a.py", contains: "x", passed: true, weight: 1.0 }],
    negative_assertions: [],
    jig_invocations: [{ command: "jig run recipe.yaml" }],
    agent_exit_code: 0,
    agent_tool_calls: 5,
    input_tokens: 12000,
    output_tokens: 3000,
    cache_creation_input_tokens: 0,
    cache_read_input_tokens: 0,
    tokens_used: 15000,
    cost_usd: 0.06,
    timeout: false,
    skills_available: true,
    tags: ["easy"],
    ...overrides,
  };
}

await test("aggregate: computes overall assertion score", () => {
  const results = [makeResult({ scores: { ...makeResult().scores, assertion_score: 0.6 } }), makeResult({ scores: { ...makeResult().scores, assertion_score: 1.0 } })];
  const agg = aggregate(results);
  assert.ok(Math.abs(agg.overall_assertion - 0.8) < 0.001);
});

await test("aggregate: computes jig_used_pct", () => {
  const results = [
    makeResult({ scores: { ...makeResult().scores, jig_used: true } }),
    makeResult({ scores: { ...makeResult().scores, jig_used: false } }),
  ];
  const agg = aggregate(results);
  assert.ok(Math.abs(agg.jig_used_pct - 0.5) < 0.001);
});

await test("aggregate: computes baseline delta", () => {
  const results = [
    makeResult({ mode: "jig", scores: { ...makeResult().scores, total: 0.9 } }),
    makeResult({ mode: "baseline", scores: { ...makeResult().scores, total: 0.7 } }),
  ];
  const agg = aggregate(results);
  assert.ok(agg.baseline_delta != null);
  assert.ok(Math.abs(agg.baseline_delta! - 0.2) < 0.001);
});

await test("aggregate: identifies weakest scenarios", () => {
  const results = [
    makeResult({ scenario: "easy-one", scores: { ...makeResult().scores, total: 1.0 } }),
    makeResult({ scenario: "hard-one", scores: { ...makeResult().scores, total: 0.2 } }),
    makeResult({ scenario: "mid-one", scores: { ...makeResult().scores, total: 0.5 } }),
  ];
  const agg = aggregate(results);
  assert.strictEqual(agg.weakest_scenarios[0].name, "hard-one");
});

await test("aggregate: by_agent breakdown", () => {
  const results = [
    makeResult({ agent: "claude-code", scores: { ...makeResult().scores, total: 0.9 } }),
    makeResult({ agent: "claude-code-sonnet", scores: { ...makeResult().scores, total: 0.7 } }),
  ];
  const agg = aggregate(results);
  assert.ok(Math.abs(agg.by_agent["claude-code"] - 0.9) < 0.001);
  assert.ok(Math.abs(agg.by_agent["claude-code-sonnet"] - 0.7) < 0.001);
});

await test("generateReport: includes all sections", () => {
  const results = [makeResult(), makeResult({ mode: "baseline", scores: { ...makeResult().scores, total: 0.6 } })];
  const report = generateReport(results);
  assert.ok(report.includes("Eval Report"));
  assert.ok(report.includes("Overall assertion score"));
  assert.ok(report.includes("By Agent"));
  assert.ok(report.includes("By Tier"));
  assert.ok(report.includes("Weakest Scenarios"));
  assert.ok(report.includes("Baseline delta"));
});

await test("generateReport: shows stddev for multi-rep runs", () => {
  const results = [
    makeResult({ rep: 1, scores: { ...makeResult().scores, total: 0.8 } }),
    makeResult({ rep: 2, scores: { ...makeResult().scores, total: 0.9 } }),
  ];
  const report = generateReport(results);
  assert.ok(report.includes("+/-"), "Should show stddev for multi-rep");
});

await test("generateMetricsOnly: outputs parseable METRIC lines", () => {
  const results = [makeResult()];
  const metrics = generateMetricsOnly(results);
  const lines = metrics.split("\n");
  for (const line of lines) {
    assert.ok(line.startsWith("METRIC "), `Line should start with METRIC: "${line}"`);
    assert.ok(line.includes("="), `Line should contain =: "${line}"`);
  }
});

await test("generateMetricsOnly: includes agent scores", () => {
  const results = [makeResult()];
  const metrics = generateMetricsOnly(results);
  assert.ok(metrics.includes("agent.claude-code="));
});

await test("aggregate: handles empty results", () => {
  const agg = aggregate([]);
  assert.strictEqual(agg.overall_assertion, 0);
  assert.strictEqual(agg.jig_used_pct, 0);
});

await test("generateReport: degrades gracefully with missing token data", () => {
  const results = [makeResult({ agent_tool_calls: 0 })];
  // Should not throw
  const report = generateReport(results);
  assert.ok(report.includes("Eval Report"));
});

// ── Prompt tier tests ──

console.log("\n--- prompt tiers ---");

await test("loadScenario: legacy prompt maps to prompts.natural", () => {
  const s = loadScenario(path.join(FIXTURES, "valid-scenario"));
  assert.ok(s.prompts.natural, "Should have natural prompt");
  assert.strictEqual(s.prompts.natural, s.prompt);
  assert.strictEqual(s.prompts.directed, undefined);
  assert.strictEqual(s.prompts.ambient, undefined);
});

await test("loadScenario: prompts map loads all three tiers", () => {
  const s = loadScenario(path.join(FIXTURES, "prompt-tiers-scenario"));
  assert.ok(s.prompts.directed?.includes("add-greeting skill"));
  assert.ok(s.prompts.natural?.includes("greeting function"));
  assert.ok(s.prompts.ambient?.includes("onboarding flow"));
  // prompt field defaults to natural
  assert.strictEqual(s.prompt, s.prompts.natural);
});

await test("validateScenario: prompts-based scenario passes validation", () => {
  const s = loadScenario(path.join(FIXTURES, "prompt-tiers-scenario"));
  const errors = validateScenario(s);
  assert.strictEqual(errors.length, 0, `Expected 0 errors, got: ${errors.map(e => e.message).join(", ")}`);
});

await test("aggregate: by_prompt_tier breakdown", () => {
  const results = [
    makeResult({ prompt_tier: "directed", scores: { ...makeResult().scores, total: 1.0 } }),
    makeResult({ prompt_tier: "natural", scores: { ...makeResult().scores, total: 0.8 } }),
    makeResult({ prompt_tier: "ambient", scores: { ...makeResult().scores, total: 0.5 } }),
  ];
  const agg = aggregate(results);
  assert.ok(Math.abs(agg.by_prompt_tier["directed"] - 1.0) < 0.001);
  assert.ok(Math.abs(agg.by_prompt_tier["natural"] - 0.8) < 0.001);
  assert.ok(Math.abs(agg.by_prompt_tier["ambient"] - 0.5) < 0.001);
});

await test("generateReport: includes By Prompt Tier section", () => {
  const results = [
    makeResult({ prompt_tier: "directed" }),
    makeResult({ prompt_tier: "natural" }),
  ];
  const report = generateReport(results);
  assert.ok(report.includes("By Prompt Tier"), "Should include prompt tier section");
  assert.ok(report.includes("directed"), "Should list directed tier");
  assert.ok(report.includes("natural"), "Should list natural tier");
});

await test("generateMetricsOnly: includes prompt tier metrics", () => {
  const results = [makeResult({ prompt_tier: "natural" })];
  const metrics = generateMetricsOnly(results);
  assert.ok(metrics.includes("prompt_tier.natural="), "Should include prompt tier metric");
});

await test("writeTrialResult + readResults: preserves prompt_tier", () => {
  const tmpFile = path.join(os.tmpdir(), `jig-test-pt-${Date.now()}.jsonl`);
  try {
    const result = makeResult({ prompt_tier: "ambient" });
    writeTrialResult(result, tmpFile);
    const read = readResults(tmpFile);
    assert.strictEqual(read[0].prompt_tier, "ambient");
  } finally {
    try { fs.unlinkSync(tmpFile); } catch {}
  }
});

// ── Summary ──

console.log(`\n--- Results: ${passed} passed, ${failed} failed ---\n`);
process.exit(failed > 0 ? 1 : 0);
