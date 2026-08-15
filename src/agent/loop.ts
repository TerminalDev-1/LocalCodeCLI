import type { Message, Provider, ToolCall, ToolExecutionResult } from "../types.js";
import { allTools, toolsByName } from "../tools/registry.js";
import type { ToolContext } from "../tools/types.js";
import { StreamingToolCallScanner, toolCallFromParsed } from "./toolCallParser.js";
import { ThinkingTagFilter, stripThinkingTags } from "./thinkingFilter.js";

const MAX_ITERATIONS = 25;

export interface AgentCallbacks {
  onTextChunk(text: string): void;
  onThinkingChunk(text: string): void;
  confirmMutatingTool(name: string, preview: string): Promise<boolean>;
  onToolStart(name: string, args: Record<string, unknown>): void;
  onToolResult(name: string, result: ToolExecutionResult): void;
  onNotice(message: string): void;
}

export interface RunAgentTurnParams {
  provider: Provider;
  model: string;
  messages: Message[];
  cwd: string;
  autoApprove: boolean;
  callbacks: AgentCallbacks;
}

/** Runs the agent until the model produces a plain-text reply with no further tool calls. */
export async function runAgentTurn(params: RunAgentTurnParams): Promise<void> {
  const { provider, model, messages, cwd, autoApprove, callbacks } = params;
  const toolDefs = allTools.map((t) => t.definition);
  const ctx: ToolContext = { cwd };

  for (let iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
    const thinkFilter = new ThinkingTagFilter();
    const scanner = new StreamingToolCallScanner();
    const fallbackCalls: ToolCall[] = [];
    let hadParseError = false;
    let fullText = "";
    let nativeToolCalls: ToolCall[] = [];

    const handleVisibleText = (text: string) => {
      for (const scanEvent of scanner.feed(text)) {
        if (scanEvent.type === "text") {
          callbacks.onTextChunk(scanEvent.text);
        } else if (scanEvent.call) {
          fallbackCalls.push(toolCallFromParsed(scanEvent.call));
        } else {
          hadParseError = true;
        }
      }
    };

    const stream = provider.chat({
      model,
      messages,
      tools: toolDefs,
      useNativeTools: true,
    });

    for await (const event of stream) {
      if (event.type === "text") {
        for (const filterEvent of thinkFilter.feed(event.text)) {
          if (filterEvent.type === "thinking") {
            callbacks.onThinkingChunk(filterEvent.text);
          } else {
            handleVisibleText(filterEvent.text);
          }
        }
      } else if (event.type === "tool_calls") {
        nativeToolCalls = event.toolCalls;
      } else if (event.type === "done") {
        fullText = event.text;
        if (event.toolCalls.length > 0) nativeToolCalls = event.toolCalls;
      }
    }

    for (const filterEvent of thinkFilter.finish()) {
      if (filterEvent.type === "thinking") {
        callbacks.onThinkingChunk(filterEvent.text);
      } else {
        handleVisibleText(filterEvent.text);
      }
    }

    for (const scanEvent of scanner.finish()) {
      if (scanEvent.type === "text") {
        callbacks.onTextChunk(scanEvent.text);
      } else if (scanEvent.call) {
        fallbackCalls.push(toolCallFromParsed(scanEvent.call));
      } else {
        hadParseError = true;
      }
    }

    const toolCalls = nativeToolCalls.length > 0 ? nativeToolCalls : fallbackCalls;

    messages.push({
      role: "assistant",
      content: stripThinkingTags(fullText),
      toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
    });

    if (toolCalls.length === 0) {
      if (hadParseError) {
        messages.push({
          role: "user",
          content:
            "Your tool_call block could not be parsed as JSON. Please retry with a single valid " +
            'JSON object: {"name": "...", "arguments": {...}}',
        });
        continue;
      }
      return; // plain-text answer, turn complete
    }

    for (const call of toolCalls) {
      const tool = toolsByName.get(call.name);

      if (!tool) {
        const result: ToolExecutionResult = {
          output: `Unknown tool "${call.name}". Available tools: ${[...toolsByName.keys()].join(", ")}`,
          isError: true,
        };
        callbacks.onToolResult(call.name, result);
        messages.push({ role: "tool", content: result.output, toolCallId: call.id, toolName: call.name });
        continue;
      }

      callbacks.onToolStart(call.name, call.arguments);

      let result: ToolExecutionResult;
      if (tool.definition.mutating && !autoApprove) {
        const preview = (await tool.preview?.(call.arguments, ctx)) ?? JSON.stringify(call.arguments);
        const approved = await callbacks.confirmMutatingTool(call.name, preview);
        result = approved
          ? await tool.execute(call.arguments, ctx)
          : { output: "User declined to run this tool.", isError: true };
      } else {
        result = await tool.execute(call.arguments, ctx);
      }

      callbacks.onToolResult(call.name, result);
      messages.push({ role: "tool", content: result.output, toolCallId: call.id, toolName: call.name });
    }
  }

  callbacks.onNotice(`Stopped after ${MAX_ITERATIONS} tool-call iterations to avoid an infinite loop.`);
}
