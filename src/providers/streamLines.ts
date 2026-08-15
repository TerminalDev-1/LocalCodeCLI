// Turns a fetch Response body into an async iterator of text lines.
// Used by both the Ollama (NDJSON) and OpenAI-compatible (SSE) adapters.

export async function* streamLines(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      let newlineIndex: number;
      while ((newlineIndex = buffer.indexOf("\n")) !== -1) {
        const line = buffer.slice(0, newlineIndex).trim();
        buffer = buffer.slice(newlineIndex + 1);
        if (line.length > 0) yield line;
      }
    }
    const rest = buffer.trim();
    if (rest.length > 0) yield rest;
  } finally {
    reader.releaseLock();
  }
}
