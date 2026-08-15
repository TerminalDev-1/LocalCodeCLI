// Shared types used across providers, tools, and the agent loop.

export type Role = "system" | "user" | "assistant" | "tool";

export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

export interface Message {
  role: Role;
  content: string;
  /** Present on assistant messages that invoked tools natively. */
  toolCalls?: ToolCall[];
  /** Present on tool-result messages; links back to the ToolCall.id. */
  toolCallId?: string;
  /** Present on tool-result messages for providers that key by name instead of id. */
  toolName?: string;
}

export interface ToolParameterSchema {
  type: "object";
  properties: Record<
    string,
    {
      type: string;
      description: string;
      items?: { type: string };
    }
  >;
  required?: string[];
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: ToolParameterSchema;
  /** Whether this tool mutates the filesystem or runs commands, and therefore needs approval. */
  mutating: boolean;
}

export interface ToolExecutionResult {
  output: string;
  isError: boolean;
}

export interface StreamTextEvent {
  type: "text";
  text: string;
}

export interface StreamToolCallEvent {
  type: "tool_calls";
  toolCalls: ToolCall[];
}

export interface StreamDoneEvent {
  type: "done";
  /** Full assistant text accumulated over the stream. */
  text: string;
  toolCalls: ToolCall[];
}

export type StreamEvent = StreamTextEvent | StreamToolCallEvent | StreamDoneEvent;

export interface ChatOptions {
  model: string;
  messages: Message[];
  tools: ToolDefinition[];
  /** Whether to also send tools in provider-native format (best-effort). Prompt-based tool calls always work regardless. */
  useNativeTools: boolean;
}

export interface Provider {
  id: string;
  /** Human-readable label, e.g. "Ollama" or "LM Studio". */
  label: string;
  /** Stream a chat completion. Yields text chunks and a final done event. */
  chat(options: ChatOptions): AsyncGenerator<StreamEvent, void, unknown>;
  /** List model names currently available from this provider, if it can be queried. */
  listModels(): Promise<string[]>;
}

export interface ProviderConfig {
  id: string;
  type: "ollama" | "openai-compatible";
  baseUrl: string;
  apiKey?: string;
  label?: string;
}

export interface LocalCodeConfig {
  providers: ProviderConfig[];
  defaultProvider: string;
  defaultModel: string;
  autoApprove: boolean;
}
