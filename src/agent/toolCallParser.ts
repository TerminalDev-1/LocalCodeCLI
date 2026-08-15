// Universal, provider-agnostic tool-calling protocol.
//
// Not every local model supports native function calling reliably (especially
// small ones), so Local Code also understands a plain-text protocol: a fenced
// code block tagged `tool_call` containing a single JSON object. This works
// with any model that can follow instructions and emit a code block, and it
// is scanned for incrementally as text streams in so the UI can render
// prose normally while hiding the raw JSON.
import { generateId } from "../utils/id.js";
import type { ToolCall } from "../types.js";

export interface ParsedToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export type ScanEvent =
  | { type: "text"; text: string }
  | { type: "tool_call"; call: ParsedToolCall | null; raw: string };

const OPEN = "```tool_call";
const CLOSE = "```";

function safeParse(content: string): ParsedToolCall | null {
  try {
    const parsed = JSON.parse(content) as { name?: unknown; arguments?: unknown };
    if (typeof parsed.name !== "string") return null;
    const args =
      parsed.arguments && typeof parsed.arguments === "object"
        ? (parsed.arguments as Record<string, unknown>)
        : {};
    return { name: parsed.name, arguments: args };
  } catch {
    return null;
  }
}

/** Incrementally scans streamed text for `tool_call` fenced blocks. */
export class StreamingToolCallScanner {
  private buffer = "";
  private cursor = 0;

  feed(chunk: string): ScanEvent[] {
    this.buffer += chunk;
    return this.drain(false);
  }

  finish(): ScanEvent[] {
    return this.drain(true);
  }

  private drain(final: boolean): ScanEvent[] {
    const events: ScanEvent[] = [];

    while (true) {
      const remaining = this.buffer.slice(this.cursor);
      const openIdx = remaining.indexOf(OPEN);

      if (openIdx === -1) {
        // Hold back a small margin in case the fence marker is split across chunks.
        const holdBack = final ? 0 : Math.min(remaining.length, OPEN.length - 1);
        const emitLen = remaining.length - holdBack;
        if (emitLen > 0) {
          events.push({ type: "text", text: remaining.slice(0, emitLen) });
          this.cursor += emitLen;
        }
        break;
      }

      if (openIdx > 0) {
        events.push({ type: "text", text: remaining.slice(0, openIdx) });
        this.cursor += openIdx;
      }

      const afterOpen = this.buffer.slice(this.cursor + OPEN.length);
      const closeIdx = afterOpen.indexOf(CLOSE);

      if (closeIdx === -1) {
        if (final) {
          events.push({ type: "text", text: this.buffer.slice(this.cursor) });
          this.cursor = this.buffer.length;
        }
        break; // wait for more input
      }

      const content = afterOpen.slice(0, closeIdx).trim();
      this.cursor += OPEN.length + closeIdx + CLOSE.length;
      events.push({ type: "tool_call", raw: content, call: safeParse(content) });
    }

    return events;
  }
}

export function toolCallFromParsed(parsed: ParsedToolCall): ToolCall {
  return { id: generateId("call"), name: parsed.name, arguments: parsed.arguments };
}
