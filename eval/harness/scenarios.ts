import fs from "node:fs";
import path from "node:path";
import { parse as parseYaml } from "yaml";
import { readdirRecursive } from "../lib/fs.ts";
import type { Assertion, NegativeAssertion, Scenario, ValidationError } from "./types.ts";

const VALID_TIERS = ["easy", "medium", "hard", "discovery", "error-recovery"] as const;

export function loadScenario(dir: string): Scenario {
  const scenarioDir = path.resolve(dir);
  const yamlPath = path.join(scenarioDir, "scenario.yaml");
  const raw = fs.readFileSync(yamlPath, "utf-8");
  const parsed = parseYaml(raw) as Record<string, unknown>;

  const assertions = ((parsed.assertions as Array<Record<string, unknown>>) ?? []).map(
    (a): Assertion => ({
      file: a.file as string,
      contains: a.contains as string,
      scope: a.scope as string | undefined,
      weight: (a.weight as number) ?? 1.0,
    })
  );

  const negativeAssertions = ((parsed.negative_assertions as Array<Record<string, unknown>>) ?? []).map(
    (a): NegativeAssertion => ({
      file: a.file as string | undefined,
      any_file: a.any_file as boolean | undefined,
      not_contains: a.not_contains as string,
      description: a.description as string | undefined,
    })
  );

  return {
    name: parsed.name as string,
    description: parsed.description as string,
    tier: parsed.tier as Scenario["tier"],
    category: (parsed.category as string) ?? "",
    prompt: parsed.prompt as string,
    context: parsed.context as string | undefined,
    expected_files_modified: (parsed.expected_files_modified as string[]) ?? [],
    assertions,
    negative_assertions: negativeAssertions.length > 0 ? negativeAssertions : undefined,
    tags: (parsed.tags as string[]) ?? [],
    estimated_jig_commands: parsed.estimated_jig_commands as number | undefined,
    max_jig_commands: parsed.max_jig_commands as number | undefined,
    scenarioDir,
  };
}

export function loadAllScenarios(baseDir: string): Scenario[] {
  const resolved = path.resolve(baseDir);
  if (!fs.existsSync(resolved)) return [];

  const entries = fs.readdirSync(resolved, { withFileTypes: true });
  const dirs = entries
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .sort(); // deterministic lexicographic order

  const scenarios: Scenario[] = [];
  for (const dir of dirs) {
    const scenarioYaml = path.join(resolved, dir, "scenario.yaml");
    if (fs.existsSync(scenarioYaml)) {
      scenarios.push(loadScenario(path.join(resolved, dir)));
    }
  }
  return scenarios;
}

export function validateScenario(scenario: Scenario): ValidationError[] {
  const errors: ValidationError[] = [];
  const sp = scenario.scenarioDir;

  const requiredStrings: Array<[string, unknown]> = [
    ["name", scenario.name],
    ["description", scenario.description],
    ["tier", scenario.tier],
    ["prompt", scenario.prompt],
  ];

  for (const [field, value] of requiredStrings) {
    if (!value || (typeof value === "string" && value.trim() === "")) {
      errors.push({ field, message: `Missing required field: ${field}`, scenarioPath: sp });
    }
  }

  if (!scenario.assertions || scenario.assertions.length === 0) {
    errors.push({ field: "assertions", message: "At least one assertion is required", scenarioPath: sp });
  }

  if (scenario.tier && !VALID_TIERS.includes(scenario.tier as (typeof VALID_TIERS)[number])) {
    errors.push({
      field: "tier",
      message: `Invalid tier "${scenario.tier}". Valid tiers: ${VALID_TIERS.join(", ")}`,
      scenarioPath: sp,
    });
  }

  // Check codebase/ exists with at least 1 file
  const codebaseDir = path.join(sp, "codebase");
  if (!fs.existsSync(codebaseDir) || !fs.statSync(codebaseDir).isDirectory()) {
    errors.push({ field: "codebase", message: "codebase/ directory must exist", scenarioPath: sp });
  } else {
    const codebaseFiles = readdirRecursive(codebaseDir);
    if (codebaseFiles.length === 0) {
      errors.push({ field: "codebase", message: "codebase/ must contain at least 1 file", scenarioPath: sp });
    }
  }

  // Check expected/ exists with at least 1 file
  const expectedDir = path.join(sp, "expected");
  if (!fs.existsSync(expectedDir) || !fs.statSync(expectedDir).isDirectory()) {
    errors.push({ field: "expected", message: "expected/ directory must exist", scenarioPath: sp });
  } else {
    const expectedFiles = readdirRecursive(expectedDir);
    if (expectedFiles.length === 0) {
      errors.push({ field: "expected", message: "expected/ must contain at least 1 file", scenarioPath: sp });
    }
  }

  // Check assertion files exist in codebase/ or expected/
  if (scenario.assertions) {
    for (const assertion of scenario.assertions) {
      const inCodebase = fs.existsSync(path.join(codebaseDir, assertion.file));
      const inExpected = fs.existsSync(path.join(expectedDir, assertion.file));
      if (!inCodebase && !inExpected) {
        errors.push({
          field: `assertions[].file`,
          message: `Assertion file "${assertion.file}" not found in codebase/ or expected/`,
          scenarioPath: sp,
        });
      }
    }
  }

  // Check expected_files_modified exist in expected/
  for (const file of scenario.expected_files_modified) {
    if (!fs.existsSync(path.join(expectedDir, file))) {
      errors.push({
        field: "expected_files_modified",
        message: `Expected file "${file}" not found in expected/`,
        scenarioPath: sp,
      });
    }
  }

  return errors;
}

