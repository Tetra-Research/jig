import type { AgentResult } from "../harness/types.ts";
import type { HeadToHeadTelemetry } from "./types.ts";

interface TelemetryExtraction {
  telemetry: HeadToHeadTelemetry;
  thinkingText?: string;
}

export function extractHeadToHeadTelemetry(
  agentResult: AgentResult,
  thinkingPrefix: string
): TelemetryExtraction {
  const lines = agentResult.stdout.split("\n");
  const events: Array<Record<string, unknown>> = [];
  const toolCallsByName: Record<string, number> = {};
  const jigInvocations: string[] = [];
  let assistantMessageCount = 0;
  let thinkingText: string | undefined;

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (line.length === 0) continue;

    let event: Record<string, unknown> | undefined;
    try {
      const parsed = JSON.parse(line) as unknown;
      if (!isObject(parsed)) continue;
      event = parsed;
      events.push(event);
    } catch {
      if (!thinkingText) {
        const found = extractThinkingFromText(line, thinkingPrefix);
        if (found) thinkingText = found;
      }
      continue;
    }

    if (event.type !== "assistant") continue;
    if (!isObject(event.message)) continue;
    assistantMessageCount++;

    const content = Array.isArray(event.message.content)
      ? (event.message.content as Array<Record<string, unknown>>)
      : [];

    for (const block of content) {
      if (!isObject(block)) continue;

      if (block.type === "text" && typeof block.text === "string" && !thinkingText) {
        const found = extractThinkingFromText(block.text, thinkingPrefix);
        if (found) thinkingText = found;
      }

      if (block.type !== "tool_use") continue;
      const toolName = typeof block.name === "string" ? block.name : "unknown";
      toolCallsByName[toolName] = (toolCallsByName[toolName] ?? 0) + 1;

      if (toolName === "Bash" && isObject(block.input) && typeof block.input.command === "string") {
        const commands = extractJigCommands(block.input.command);
        jigInvocations.push(...commands);
      }
    }
  }

  const initEvent = events.find((event) => event.type === "system" && event.subtype === "init");
  const resultEvent = [...events].reverse().find((event) => event.type === "result");

  const usage = isObject(resultEvent?.usage) ? resultEvent.usage : {};
  const inputTokens = asNumber(usage.input_tokens);
  const outputTokens = asNumber(usage.output_tokens);
  const cacheCreationInputTokens = asNumber(usage.cache_creation_input_tokens);
  const cacheReadInputTokens = asNumber(usage.cache_read_input_tokens);

  const contextTokens = sumDefined(inputTokens, cacheCreationInputTokens, cacheReadInputTokens);
  const tokensUsed = sumDefined(inputTokens, outputTokens, cacheCreationInputTokens, cacheReadInputTokens);

  const telemetry: HeadToHeadTelemetry = {
    model: stringOrUndefined(initEvent?.model) ?? deriveModelFromResult(resultEvent),
    service_tier: stringOrUndefined(usage.service_tier),
    duration_api_ms: asNumber(resultEvent?.duration_api_ms),
    num_turns: asNumber(resultEvent?.num_turns) ?? 0,
    tool_calls: Object.values(toolCallsByName).reduce((sum, value) => sum + value, 0),
    tool_calls_by_name: toolCallsByName,
    jig_invocation_count: jigInvocations.length,
    jig_invocations: jigInvocations,
    assistant_message_count: assistantMessageCount,
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    cache_creation_input_tokens: cacheCreationInputTokens,
    cache_read_input_tokens: cacheReadInputTokens,
    context_tokens: contextTokens,
    tokens_used: tokensUsed,
    cost_usd: asNumber(resultEvent?.total_cost_usd),
    permission_denials_count: Array.isArray(resultEvent?.permission_denials)
      ? resultEvent.permission_denials.length
      : 0,
    model_usage: isObject(resultEvent?.modelUsage) ? resultEvent.modelUsage : undefined,
    init_event: isObject(initEvent) ? initEvent : undefined,
    result_event: isObject(resultEvent) ? resultEvent : undefined,
  };

  return {
    telemetry,
    thinkingText,
  };
}

function extractThinkingFromText(text: string, prefix: string): string | undefined {
  const normalized = text.trim();
  if (!normalized.startsWith(prefix)) return undefined;
  return normalized.slice(prefix.length).trim();
}

function extractJigCommands(commandText: string): string[] {
  const out: string[] = [];
  const chunks = commandText
    .split("\n")
    .map((chunk) => chunk.trim())
    .filter((chunk) => chunk.length > 0);

  for (const chunk of chunks) {
    const matches = chunk.match(/\bjig\s+(run|workflow|render|list)\b[^\n]*/g);
    if (!matches) continue;
    out.push(...matches.map((match) => match.trim()));
  }
  return out;
}

function deriveModelFromResult(resultEvent: Record<string, unknown> | undefined): string | undefined {
  if (!isObject(resultEvent?.modelUsage)) return undefined;
  const keys = Object.keys(resultEvent.modelUsage);
  if (keys.length === 0) return undefined;
  return keys[0];
}

function sumDefined(...values: Array<number | undefined>): number | undefined {
  if (values.every((value) => value == null)) return undefined;
  return values.reduce((sum, value) => sum + (value ?? 0), 0);
}

function asNumber(value: unknown): number | undefined {
  if (typeof value !== "number" || !Number.isFinite(value)) return undefined;
  return value;
}

function stringOrUndefined(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function isObject(value: unknown): value is Record<string, any> {
  return typeof value === "object" && value !== null;
}
