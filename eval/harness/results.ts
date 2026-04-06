import fs from "node:fs";
import path from "node:path";
import type {
  TrialResult,
  ReadResultsOutput,
  ResultReadDiagnostics,
  ResultReadEntry,
  ResultReadWarning,
  ResultSchemaVersion,
  SchemaPolicyMode,
} from "./types.ts";

export function writeTrialResult(result: TrialResult, filePath: string): void {
  try {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.appendFileSync(filePath, JSON.stringify(result) + "\n");
  } catch (err) {
    console.error(`[eval] Failed to write trial result to ${filePath}: ${err}`);
  }
}

const DETAILED_EFFICIENCY_KEYS = [
  "input_tokens",
  "output_tokens",
  "cache_creation_input_tokens",
  "cache_read_input_tokens",
] as const;
const TOTAL_EFFICIENCY_KEYS = ["tokens_used", "cost_usd"] as const;
const ALL_EFFICIENCY_KEYS = [...DETAILED_EFFICIENCY_KEYS, ...TOTAL_EFFICIENCY_KEYS] as const;

export interface ReadResultsOptions {
  schemaMode?: SchemaPolicyMode;
}

export class ResultSchemaError extends Error {
  readonly diagnostics: ResultReadDiagnostics;

  constructor(message: string, diagnostics: ResultReadDiagnostics) {
    super(message);
    this.name = "ResultSchemaError";
    this.diagnostics = diagnostics;
  }
}

export function readResults(filePath: string, options: ReadResultsOptions = {}): ReadResultsOutput {
  const schemaMode = options.schemaMode ?? "strict";
  if (!fs.existsSync(filePath)) {
    return {
      results: [],
      entries: [],
      diagnostics: emptyDiagnostics(filePath),
    };
  }

  const raw = fs.readFileSync(filePath, "utf-8");
  if (raw.trim() === "") {
    return {
      results: [],
      entries: [],
      diagnostics: emptyDiagnostics(filePath),
    };
  }

  const entries: ResultReadEntry[] = [];
  const warnings: ResultReadWarning[] = [];
  const lines = raw.split("\n");
  let totalLines = 0;
  let malformedLines = 0;
  let invalidRows = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === "") continue;
    totalLines++;

    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch {
      malformedLines++;
      warnings.push({
        line: i + 1,
        kind: "malformed_json",
        message: "Malformed JSON line",
        preview: line.slice(0, 120),
      });
      continue;
    }

    const schema = classifySchema(parsed);
    if (schema === "invalid") {
      invalidRows++;
      warnings.push({
        line: i + 1,
        kind: "invalid_schema",
        message: "Row does not match a supported eval result schema",
        preview: line.slice(0, 120),
      });
      continue;
    }

    entries.push({
      line: i + 1,
      schema,
      result: parsed as TrialResult,
    });
  }

  const schemaCounts: Record<ResultSchemaVersion, number> = {
    v1_legacy: entries.filter((entry) => entry.schema === "v1_legacy").length,
    v2_current: entries.filter((entry) => entry.schema === "v2_current").length,
  };

  const diagnostics: ResultReadDiagnostics = {
    file_path: filePath,
    total_lines: totalLines,
    valid_rows: entries.length,
    malformed_lines: malformedLines,
    invalid_rows: invalidRows,
    schema_counts: schemaCounts,
    is_mixed_schema: schemaCounts.v1_legacy > 0 && schemaCounts.v2_current > 0,
    warnings,
  };

  if (schemaMode === "strict") {
    const violations: string[] = [];
    if (diagnostics.malformed_lines > 0) {
      violations.push(`${diagnostics.malformed_lines} malformed JSONL line(s)`);
    }
    if (diagnostics.invalid_rows > 0) {
      violations.push(`${diagnostics.invalid_rows} invalid schema row(s)`);
    }
    if (diagnostics.is_mixed_schema) {
      violations.push("mixed schema rows (v1_legacy + v2_current)");
    }

    if (violations.length > 0) {
      const message =
        `[eval] Strict schema validation failed for ${filePath}: ${violations.join("; ")}. ` +
        `Use --schema-mode compat to inspect mixed or malformed archives safely.`;
      throw new ResultSchemaError(message, diagnostics);
    }
  }

  return {
    results: entries.map((entry) => entry.result),
    entries,
    diagnostics,
  };
}

export function hasCompleteEfficiencyMetrics(result: TrialResult): boolean {
  return ALL_EFFICIENCY_KEYS.every((key) => typeof result[key] === "number");
}

export function formatDiagnosticsSummary(diagnostics: ResultReadDiagnostics): string[] {
  const lines: string[] = [];
  lines.push(
    `[eval] Results schema summary: valid=${diagnostics.valid_rows}/${diagnostics.total_lines}, ` +
    `v1_legacy=${diagnostics.schema_counts.v1_legacy}, v2_current=${diagnostics.schema_counts.v2_current}, ` +
    `malformed=${diagnostics.malformed_lines}, invalid=${diagnostics.invalid_rows}`
  );

  if (diagnostics.is_mixed_schema) {
    lines.push("[eval] Mixed schema detected: efficiency metrics are computed only on fully covered rows.");
  }

  for (const warning of diagnostics.warnings.slice(0, 5)) {
    lines.push(`[eval] Warning line ${warning.line}: ${warning.message} (${warning.preview})`);
  }
  if (diagnostics.warnings.length > 5) {
    lines.push(`[eval] ... plus ${diagnostics.warnings.length - 5} additional warning(s)`);
  }

  return lines;
}

function emptyDiagnostics(filePath: string): ResultReadDiagnostics {
  return {
    file_path: filePath,
    total_lines: 0,
    valid_rows: 0,
    malformed_lines: 0,
    invalid_rows: 0,
    schema_counts: {
      v1_legacy: 0,
      v2_current: 0,
    },
    is_mixed_schema: false,
    warnings: [],
  };
}

function classifySchema(value: unknown): ResultSchemaVersion | "invalid" {
  if (!isBaseTrialResult(value)) return "invalid";

  const obj = value as Record<string, unknown>;
  const hasAnyDetailedKey = DETAILED_EFFICIENCY_KEYS.some((key) => hasOwn(obj, key));
  const hasAllDetailedNumeric = DETAILED_EFFICIENCY_KEYS.every((key) => isNumber(obj[key]));
  const hasAllTotalsNumeric = TOTAL_EFFICIENCY_KEYS.every((key) => isNumber(obj[key]));

  if (hasAllDetailedNumeric && hasAllTotalsNumeric) {
    return "v2_current";
  }

  if (!hasAnyDetailedKey) {
    // Legacy rows may include tokens_used/cost_usd, or may omit efficiency entirely.
    if (efficiencyKeysHaveInvalidTypes(obj)) return "invalid";
    return "v1_legacy";
  }

  // Partial detailed efficiency fields are considered invalid because they
  // cannot be safely interpreted for aggregate efficiency metrics.
  return "invalid";
}

function efficiencyKeysHaveInvalidTypes(obj: Record<string, unknown>): boolean {
  for (const key of ALL_EFFICIENCY_KEYS) {
    if (!hasOwn(obj, key)) continue;
    const value = obj[key];
    if (value == null) continue;
    if (!isNumber(value)) return true;
  }
  return false;
}

function isBaseTrialResult(value: unknown): value is TrialResult {
  if (!isObject(value)) return false;
  const obj = value as Record<string, unknown>;
  if (!isString(obj.scenario)) return false;
  if (!isString(obj.agent)) return false;
  if (obj.mode !== "jig" && obj.mode !== "baseline") return false;
  if (!isNumber(obj.rep)) return false;
  if (!isString(obj.tier)) return false;
  if (!isString(obj.category)) return false;
  if (!isString(obj.timestamp)) return false;
  if (!isNumber(obj.duration_ms)) return false;
  if (!isString(obj.jig_version)) return false;
  if (!isObject(obj.scores)) return false;

  const scores = obj.scores as Record<string, unknown>;
  if (!isNumber(scores.assertion_score)) return false;
  if (!isNumber(scores.file_score)) return false;
  if (!isNumber(scores.negative_score)) return false;
  if (typeof scores.jig_used !== "boolean") return false;
  if (typeof scores.jig_correct !== "boolean") return false;
  if (!isNumber(scores.total)) return false;

  if (!Array.isArray(obj.assertions)) return false;
  if (!Array.isArray(obj.negative_assertions)) return false;
  if (!Array.isArray(obj.jig_invocations)) return false;
  if (!isNumber(obj.agent_exit_code)) return false;
  if (!isNumber(obj.agent_tool_calls)) return false;
  if (typeof obj.timeout !== "boolean") return false;
  return true;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function hasOwn(obj: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(obj, key);
}
