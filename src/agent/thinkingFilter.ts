// Some local reasoning models (DeepSeek-R1, QwQ, Qwen3 thinking mode, ...) emit
// their chain-of-thought inline as <think>...</think> before the real answer.
// This strips those tags from the text stream, so raw markup never leaks into
// the terminal — callers can still choose to render "thinking" text dimmed.

export type FilterEvent = { type: "thinking" | "text"; text: string };

const OPEN = "<think>";
const CLOSE = "</think>";

export class ThinkingTagFilter {
  private buffer = "";
  private cursor = 0;
  private inThinking = false;

  feed(chunk: string): FilterEvent[] {
    this.buffer += chunk;
    return this.drain(false);
  }

  finish(): FilterEvent[] {
    return this.drain(true);
  }

  private drain(final: boolean): FilterEvent[] {
    const events: FilterEvent[] = [];
    const marker = this.inThinking ? CLOSE : OPEN;

    while (true) {
      const remaining = this.buffer.slice(this.cursor);
      const idx = remaining.indexOf(marker);

      if (idx === -1) {
        const holdBack = final ? 0 : Math.min(remaining.length, marker.length - 1);
        const emitLen = remaining.length - holdBack;
        if (emitLen > 0) {
          events.push({ type: this.inThinking ? "thinking" : "text", text: remaining.slice(0, emitLen) });
          this.cursor += emitLen;
        }
        return events;
      }

      if (idx > 0) {
        events.push({ type: this.inThinking ? "thinking" : "text", text: remaining.slice(0, idx) });
      }
      this.cursor += idx + marker.length;
      this.inThinking = !this.inThinking;
      return [...events, ...this.drain(final)];
    }
  }
}

/** One-shot strip for text that's already fully accumulated (e.g. before saving to history). */
export function stripThinkingTags(text: string): string {
  return text.replace(/<think>[\s\S]*?<\/think>/g, "").trim();
}
