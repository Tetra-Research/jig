import assert from "node:assert";
import { extractHeadToHeadTelemetry } from "./telemetry.ts";
import type { AgentResult } from "../harness/types.ts";

function makeAgentResult(stdout: string): AgentResult {
  return {
    agent: "test-agent",
    exitCode: 0,
    stdout,
    stderr: "",
    durationMs: 1_000,
    timedOut: false,
  };
}

const thinkingPrefix = "HEAD2HEAD_THINKING:";

const streamJsonOutput = [
  JSON.stringify({
    type: "system",
    subtype: "init",
    model: "claude-opus-4-6",
  }),
  JSON.stringify({
    type: "assistant",
    message: {
      content: [
        { type: "text", text: "HEAD2HEAD_THINKING: I will inspect files then apply a small patch." },
        { type: "tool_use", name: "Bash", input: { command: "jig run recipe.yaml --vars '{\"a\":1}'" } },
      ],
    },
  }),
  JSON.stringify({
    type: "result",
    duration_api_ms: 1234,
    num_turns: 7,
    total_cost_usd: 0.42,
    permission_denials: [],
    usage: {
      input_tokens: 10,
      output_tokens: 20,
      cache_creation_input_tokens: 30,
      cache_read_input_tokens: 40,
      service_tier: "standard",
    },
    modelUsage: {
      "claude-opus-4-6": {
        inputTokens: 10,
      },
    },
  }),
].join("\n");

const extracted = extractHeadToHeadTelemetry(makeAgentResult(streamJsonOutput), thinkingPrefix);
assert.strictEqual(extracted.thinkingText, "I will inspect files then apply a small patch.");
assert.strictEqual(extracted.telemetry.model, "claude-opus-4-6");
assert.strictEqual(extracted.telemetry.num_turns, 7);
assert.strictEqual(extracted.telemetry.tool_calls, 1);
assert.strictEqual(extracted.telemetry.jig_invocation_count, 1);
assert.strictEqual(extracted.telemetry.context_tokens, 80);
assert.strictEqual(extracted.telemetry.tokens_used, 100);
assert.strictEqual(extracted.telemetry.cost_usd, 0.42);

const plainTextOutput = `${thinkingPrefix} quick plan`;
const extractedPlain = extractHeadToHeadTelemetry(makeAgentResult(plainTextOutput), thinkingPrefix);
assert.strictEqual(extractedPlain.thinkingText, "quick plan");
assert.strictEqual(extractedPlain.telemetry.tool_calls, 0);

console.log("head2head telemetry tests passed");
