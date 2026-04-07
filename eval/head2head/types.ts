import type {
  AgentArtifactPaths,
  AgentConfig,
  AssertionResult,
  NegativeAssertionResult,
  Scenario,
  TrialScore,
} from "../harness/types.ts";

export type HeadToHeadArm = "control" | "jig";
export type HeadToHeadPromptSource = "natural" | "directed" | "ambient" | "legacy_prompt" | "custom";

export interface HeadToHeadArmConfig {
  arm: HeadToHeadArm;
  label: string;
  profilePath: string;
}

export interface HeadToHeadRunConfig {
  scenarios: Scenario[];
  agent: AgentConfig;
  reps: number;
  arms: [HeadToHeadArmConfig, HeadToHeadArmConfig];
  promptSource: HeadToHeadPromptSource;
  promptText?: string;
  thinkingMode: boolean;
  resultsPath: string;
  pairsPath: string;
  artifactsRoot: string;
  captureArtifacts: boolean;
  cleanSlate: boolean;
}

export interface HeadToHeadSandbox {
  workDir: string;
  jigVersion: string;
  profilePath: string;
  installedSkills: string[];
  hasClaudeMd: boolean;
  cleanup: () => Promise<void>;
}

export interface HeadToHeadTelemetry {
  model?: string;
  service_tier?: string;
  duration_api_ms?: number;
  num_turns: number;
  tool_calls: number;
  tool_calls_by_name: Record<string, number>;
  jig_invocation_count: number;
  jig_invocations: string[];
  assistant_message_count: number;
  input_tokens?: number;
  output_tokens?: number;
  cache_creation_input_tokens?: number;
  cache_read_input_tokens?: number;
  context_tokens?: number;
  tokens_used?: number;
  cost_usd?: number;
  permission_denials_count: number;
  model_usage?: Record<string, unknown>;
  init_event?: Record<string, unknown>;
  result_event?: Record<string, unknown>;
}

export interface HeadToHeadTrialResult {
  schema_version: "head2head_v1";
  timestamp: string;
  run_id: string;
  scenario: string;
  rep: number;
  agent: string;
  arm: HeadToHeadArm;
  arm_label: string;
  profile_path: string;
  prompt_source: HeadToHeadPromptSource;
  thinking_mode: boolean;
  thinking_text?: string;
  tier: string;
  category: string;
  tags: string[];
  duration_ms: number;
  jig_version: string;
  installed_skills: string[];
  has_claude_md: boolean;
  scores: TrialScore;
  file_score: number;
  assertions: AssertionResult[];
  negative_assertions: NegativeAssertionResult[];
  jig_invocations: Array<{ command: string; vars?: string; exit_code?: number }>;
  agent_exit_code: number;
  timeout: boolean;
  telemetry: HeadToHeadTelemetry;
  agent_artifacts?: AgentArtifactPaths;
}

export interface NumericDelta {
  control: number;
  jig: number;
  abs_delta: number;
  pct_delta?: number;
}

export interface HeadToHeadPairResult {
  schema_version: "head2head_pair_v1";
  timestamp: string;
  run_id: string;
  scenario: string;
  rep: number;
  agent: string;
  score?: NumericDelta;
  file_score?: NumericDelta;
  duration_ms?: NumericDelta;
  tool_calls?: NumericDelta;
  context_tokens?: NumericDelta;
  output_tokens?: NumericDelta;
  tokens_used?: NumericDelta;
  cost_usd?: NumericDelta;
}
