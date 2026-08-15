import type {
  ChatOptions,
  Message,
  Provider,
  ProviderConfig,
  StreamEvent,
  ToolCall,
} from "../types.js";
import { streamLines } from "./streamLines.js";

interface OllamaToolCall {
  function: {
    name: string;
    arguments: Record<string, unknown>;
  };
}

interface OllamaChatChunk {
  message?: {
    role: string;
    content: string;
    tool_calls?: OllamaToolCall[];
  };
  done: boolean;
}

function toOllamaMessages(messages: Message[]): unknown[] {
  return messages.map((m) => {
    if (m.role === "tool") {
      // Ollama expects tool results as role "tool" with plain text content.
      return { role: "tool", content: m.content };
    }
    return { role: m.role, content: m.content };
  });
}

function toOllamaTools(tools: ChatOptions["tools"]): unknown[] {
  return tools.map((t) => ({
    type: "function",
    function: {
      name: t.name,
      description: t.description,
      parameters: t.parameters,
    },
  }));
}

let callCounter = 0;
function nextToolCallId(): string {
  callCounter += 1;
  return `call_${Date.now()}_${callCounter}`;
}

export function createOllamaProvider(config: ProviderConfig): Provider {
  const baseUrl = config.baseUrl.replace(/\/$/, "");

  return {
    id: config.id,
    label: config.label ?? "Ollama",

    async *chat(options: ChatOptions): AsyncGenerator<StreamEvent, void, unknown> {
      const res = await fetch(`${baseUrl}/api/chat`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          model: options.model,
          messages: toOllamaMessages(options.messages),
          stream: true,
          ...(options.useNativeTools && options.tools.length > 0
            ? { tools: toOllamaTools(options.tools) }
            : {}),
        }),
      });

      if (!res.ok || !res.body) {
        const body = await res.text().catch(() => "");
        throw new Error(`Ollama request failed (${res.status}): ${body || res.statusText}`);
      }

      let fullText = "";
      const toolCalls: ToolCall[] = [];

      for await (const line of streamLines(res.body)) {
        let chunk: OllamaChatChunk;
        try {
          chunk = JSON.parse(line) as OllamaChatChunk;
        } catch {
          continue;
        }

        const content = chunk.message?.content ?? "";
        if (content) {
          fullText += content;
          yield { type: "text", text: content };
        }

        if (chunk.message?.tool_calls && chunk.message.tool_calls.length > 0) {
          const parsed = chunk.message.tool_calls.map((tc) => ({
            id: nextToolCallId(),
            name: tc.function.name,
            arguments: tc.function.arguments ?? {},
          }));
          toolCalls.push(...parsed);
          yield { type: "tool_calls", toolCalls: parsed };
        }

        if (chunk.done) break;
      }

      yield { type: "done", text: fullText, toolCalls };
    },

    async listModels(): Promise<string[]> {
      const res = await fetch(`${baseUrl}/api/tags`);
      if (!res.ok) return [];
      const data = (await res.json()) as { models?: { name: string }[] };
      return (data.models ?? []).map((m) => m.name);
    },
  };
}
