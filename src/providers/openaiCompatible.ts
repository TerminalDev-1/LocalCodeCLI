import type {
  ChatOptions,
  Message,
  Provider,
  ProviderConfig,
  StreamEvent,
  ToolCall,
} from "../types.js";
import { streamLines } from "./streamLines.js";

interface OpenAiDeltaToolCall {
  index: number;
  id?: string;
  function?: {
    name?: string;
    arguments?: string;
  };
}

interface OpenAiChunk {
  choices?: {
    delta?: {
      content?: string;
      tool_calls?: OpenAiDeltaToolCall[];
    };
    finish_reason?: string | null;
  }[];
}

function toOpenAiMessages(messages: Message[]): unknown[] {
  return messages.map((m) => {
    if (m.role === "tool") {
      return {
        role: "tool",
        tool_call_id: m.toolCallId ?? "call_unknown",
        content: m.content,
      };
    }
    if (m.role === "assistant" && m.toolCalls && m.toolCalls.length > 0) {
      return {
        role: "assistant",
        content: m.content,
        tool_calls: m.toolCalls.map((tc) => ({
          id: tc.id,
          type: "function",
          function: { name: tc.name, arguments: JSON.stringify(tc.arguments) },
        })),
      };
    }
    return { role: m.role, content: m.content };
  });
}

function toOpenAiTools(tools: ChatOptions["tools"]): unknown[] {
  return tools.map((t) => ({
    type: "function",
    function: {
      name: t.name,
      description: t.description,
      parameters: t.parameters,
    },
  }));
}

export function createOpenAiCompatibleProvider(config: ProviderConfig): Provider {
  const baseUrl = config.baseUrl.replace(/\/$/, "");

  return {
    id: config.id,
    label: config.label ?? "OpenAI-compatible",

    async *chat(options: ChatOptions): AsyncGenerator<StreamEvent, void, unknown> {
      const res = await fetch(`${baseUrl}/chat/completions`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          ...(config.apiKey ? { authorization: `Bearer ${config.apiKey}` } : {}),
        },
        body: JSON.stringify({
          model: options.model,
          messages: toOpenAiMessages(options.messages),
          stream: true,
          ...(options.useNativeTools && options.tools.length > 0
            ? { tools: toOpenAiTools(options.tools) }
            : {}),
        }),
      });

      if (!res.ok || !res.body) {
        const body = await res.text().catch(() => "");
        throw new Error(
          `${config.label ?? "OpenAI-compatible"} request failed (${res.status}): ${
            body || res.statusText
          }`,
        );
      }

      let fullText = "";
      const toolCallBuffers = new Map<
        number,
        { id: string; name: string; args: string }
      >();

      for await (const line of streamLines(res.body)) {
        if (!line.startsWith("data:")) continue;
        const payload = line.slice("data:".length).trim();
        if (payload === "[DONE]") break;

        let chunk: OpenAiChunk;
        try {
          chunk = JSON.parse(payload) as OpenAiChunk;
        } catch {
          continue;
        }

        const delta = chunk.choices?.[0]?.delta;
        if (delta?.content) {
          fullText += delta.content;
          yield { type: "text", text: delta.content };
        }

        if (delta?.tool_calls) {
          for (const tc of delta.tool_calls) {
            const existing = toolCallBuffers.get(tc.index) ?? {
              id: tc.id ?? `call_${tc.index}`,
              name: "",
              args: "",
            };
            if (tc.id) existing.id = tc.id;
            if (tc.function?.name) existing.name += tc.function.name;
            if (tc.function?.arguments) existing.args += tc.function.arguments;
            toolCallBuffers.set(tc.index, existing);
          }
        }
      }

      const toolCalls: ToolCall[] = [...toolCallBuffers.values()].map((tc) => {
        let args: Record<string, unknown> = {};
        try {
          args = tc.args ? (JSON.parse(tc.args) as Record<string, unknown>) : {};
        } catch {
          args = {};
        }
        return { id: tc.id, name: tc.name, arguments: args };
      });

      if (toolCalls.length > 0) {
        yield { type: "tool_calls", toolCalls };
      }

      yield { type: "done", text: fullText, toolCalls };
    },

    async listModels(): Promise<string[]> {
      try {
        const res = await fetch(`${baseUrl}/models`, {
          headers: config.apiKey ? { authorization: `Bearer ${config.apiKey}` } : {},
        });
        if (!res.ok) return [];
        const data = (await res.json()) as { data?: { id: string }[] };
        return (data.data ?? []).map((m) => m.id);
      } catch {
        return [];
      }
    },
  };
}
