# Local Code

A coding agent, built from scratch, that works with any model you point it at — local or
cloud. It reads files, edits them, runs shell commands, and iterates in a loop until the
task is done, same shape as tools like Claude Code or OpenCode, but provider-agnostic from
the ground up. Local Code is a native Windows desktop app, written in Rust with
[iced](https://iced.rs) — see [`desktop/`](desktop) for the full breakdown of its layout.

Two backends ship out of the box:

- **Ollama** — `http://localhost:11434`
- **LM Studio** (or any OpenAI-compatible server: llama.cpp server, vLLM, LocalAI, text-generation-webui) — `http://localhost:1234/v1`

## Why it works with small models too

Not every local model supports native function calling reliably, especially small ones.
Local Code doesn't depend on that: alongside native tool calls, it teaches every model a
plain-text protocol via the system prompt — a fenced ` ```tool_call ` block containing a
single JSON object — and scans the response for it as it streams in. If the model supports
native tool calling, that's used automatically. If not, the text protocol still gets you a
working agent loop. Reasoning models that emit `<think>...</think>` blocks (DeepSeek-R1,
QwQ, Qwen thinking mode) are also handled — their thinking is filtered out of the
conversation history sent back to the model.

## Build & run

Requires a Rust toolchain (`rustup`).

```bash
cd desktop
cargo build -p local_code_desktop
cargo run -p local_code_desktop
```

The window is undecorated with a custom title bar (drag to move, edge/corner regions to
resize, minimize/maximize/close in the top-right) instead of the OS chrome.

## Quick start

Start [Ollama](https://ollama.com) or [LM Studio](https://lmstudio.ai) and load a model, then
launch the app. If no default model is configured, launching it starts a setup wizard: it
probes configured providers, falls back to checking for a local Ollama install (offering to
start it, or pull a hardware-sized model recommendation), or otherwise walks manual
provider/model selection.

## Configuration

Config lives at `~/.local-code/config.json`, with an optional per-project override at
`.local-code.json` in the project's directory (both are merged, project wins). Shape:

```json
{
  "providers": [
    { "id": "ollama", "type": "ollama", "baseUrl": "http://localhost:11434" },
    { "id": "lmstudio", "type": "openai-compatible", "baseUrl": "http://localhost:1234/v1" },
    { "id": "openai", "type": "openai-compatible", "baseUrl": "https://api.openai.com/v1", "apiKey": "sk-..." }
  ],
  "defaultProvider": "ollama",
  "defaultModel": "llama3.1:8b",
  "autoApprove": false
}
```

Any OpenAI-compatible endpoint works by adding a provider with `"type": "openai-compatible"`
and its `baseUrl` (and `apiKey` if it needs one) — that covers cloud APIs too, not just local ones.

Projects and chats are the desktop app's own state, stored separately at
`~/.local-code/desktop/workspace.json`.

## Tools available to the agent

`read_file`, `write_file`, `edit_file` (exact string replace), `list_dir`, `glob`, `grep`, `bash`.

## Project layout

See [`desktop/README.md`](desktop/README.md) for the full breakdown of `desktop/core` and
`desktop/app`.
