// ── Scenarios ──

export type PromptTier = "directed" | "natural" | "ambient";
export type ClaudeMdMode = "shared" | "empty" | "none";

export interface Scenario {
  name: string;
  description: string;
  tier: "easy" | "medium" | "hard" | "discovery";
  category: string;
  prompt: string;
  prompts: Partial<Record<PromptTier, string>>;
  context?: string;
  expected_files_modified: string[];
  assertions: Assertion[];
  negative_assertions?: NegativeAssertion[];
  tags?: string[];
  estimated_jig_commands?: number;
  max_jig_commands?: number;
  scenarioDir: string;
}

export interface Assertion {
  file: string;
  contains: string;
  ordered_contains?: string[];
  scope?: string;
  weight: number;
}

export interface NegativeAssertion {
  file?: string;
  any_file?: boolean;
  not_contains: string;
  description?: string;
}

// ── Agents ──

export interface AgentConfig {
  name: string;
  command: string;
  args: string[];
  timeout_ms: number;
  env?: Record<string, string>;
}

export interface AgentResult {
  agent: string;
  exitCode: number;
  stdout: string;
  stderr: string;
  durationMs: number;
  timedOut: boolean;
}

// ── Sandbox ──

export interface Sandbox {
  workDir: string;
  jigVersion: string;
  skillsAvailable: boolean;
  jigShimDir?: string;
  cleanup: () => Promise<void>;
}

// ── Scoring ──

export interface TrialScore {
  assertion_score: number;
  file_score: number;
  negative_score: number;
  jig_used: boolean;
  jig_correct: boolean;
  total: number;
}

export interface AssertionResult {
  file: string;
  contains: string;
  ordered_contains?: string[];
  scope?: string;
  passed: boolean;
  weight: number;
}

export interface NegativeAssertionResult {
  file?: string;
  any_file?: boolean;
  not_contains: string;
  passed: boolean;
  description?: string;
}

export interface JigInvocation {
  command: string;
  vars?: string;
  exit_code?: number;
}

export interface AgentArtifactPaths {
  dir: string;
  prompt: string;
  stdout: string;
  stderr: string;
  combined: string;
  git_status?: string;
  diff_stat?: string;
  diff_patch?: string;
  changed_files_manifest?: string;
  workspace_snapshot_dir?: string;
}

// ── Results ──

export interface TrialResult {
  scenario: string;
  agent: string;
  mode: "jig" | "baseline";
  prompt_tier?: PromptTier;
  claude_md?: ClaudeMdMode;
  rep: number;
  tier: string;
  category: string;
  timestamp: string;
  duration_ms: number;
  jig_version: string;
  scores: TrialScore;
  assertions: AssertionResult[];
  negative_assertions: NegativeAssertionResult[];
  jig_invocations: JigInvocation[];
  agent_exit_code: number;
  agent_tool_calls: number;
  input_tokens?: number;
  output_tokens?: number;
  cache_creation_input_tokens?: number;
  cache_read_input_tokens?: number;
  tokens_used?: number;
  cost_usd?: number;
  timeout: boolean;
  skills_available?: boolean;
  tags?: string[];
  agent_artifacts?: AgentArtifactPaths;
}

// ── Reporting ──

export interface EfficiencyCoverage {
  covered: number;
  total: number;
}

export interface AggregateScores {
  overall_assertion: number;
  jig_used_pct: number;
  baseline_delta?: number;
  by_agent: Record<string, number>;
  by_tier: Record<string, number>;
  by_prompt_tier: Record<string, number>;
  by_category: Record<string, number>;
  weakest_scenarios: Array<{ name: string; score: number }>;
  efficiency_coverage_all: EfficiencyCoverage;
  efficiency_coverage_jig: EfficiencyCoverage;
  efficiency_coverage_baseline: EfficiencyCoverage;
  mean_duration_jig?: number;
  mean_duration_baseline?: number;
  mean_tokens_jig?: number;
  mean_tokens_baseline?: number;
  mean_input_tokens_jig?: number;
  mean_input_tokens_baseline?: number;
  mean_output_tokens_jig?: number;
  mean_output_tokens_baseline?: number;
  mean_cost_jig?: number;
  mean_cost_baseline?: number;
}

// ── Validation ──

export interface ValidationError {
  field: string;
  message: string;
  scenarioPath: string;
}

// ── Results ingestion ──

export type ResultSchemaVersion = "v1_legacy" | "v2_current";
export type SchemaPolicyMode = "strict" | "compat";

export interface ResultReadWarning {
  line: number;
  kind: "malformed_json" | "invalid_schema";
  message: string;
  preview: string;
}

export interface ResultReadDiagnostics {
  file_path: string;
  total_lines: number;
  valid_rows: number;
  malformed_lines: number;
  invalid_rows: number;
  schema_counts: Record<ResultSchemaVersion, number>;
  is_mixed_schema: boolean;
  warnings: ResultReadWarning[];
}

export interface ResultReadEntry {
  line: number;
  schema: ResultSchemaVersion;
  result: TrialResult;
}

export interface ReadResultsOutput {
  results: TrialResult[];
  entries: ResultReadEntry[];
  diagnostics: ResultReadDiagnostics;
}
