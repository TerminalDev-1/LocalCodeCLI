# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Local Code is a coding agent that works with any model — local or cloud — behind an Ollama
or OpenAI-compatible endpoint. It reads/edits files, runs shell commands, and loops with the
model until a task is done.

It's a native Windows GUI, written in Rust with [iced](https://iced.rs), split into
`desktop/core` (crate `local_code_core`: providers, tools, agent loop — UI-agnostic) and
`desktop/app` (crate `local_code_desktop`: the iced application).

## Commands

```bash
cd desktop
cargo build -p local_code_desktop   # or: cargo build -p local_code_core
cargo run -p local_code_desktop
cargo check -p local_code_desktop   # faster, no codegen
```

No `#[test]`s exist in `desktop/` yet.

Two things that bite repeatedly in this environment:

- `cargo` is often not on `PATH` in a fresh shell here; if `cargo build` reports "not
  recognized", call it via the full rustup path instead:
  `& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p local_code_desktop`.
- A running `local_code_desktop.exe` locks its own binary. `cargo build` will fail with
  "Access is denied" (os error 5) removing the old exe if the app is still open — stop the
  process first (`Get-Process local_code_desktop | Stop-Process -Force`).

## Architecture

### Provider abstraction

`core/src/providers` defines a `Provider` trait with `chat(...)` (streams `StreamEvent`s:
text chunks, native tool calls, a final done event) and `list_models()`. Two implementations
ship:

- **Ollama** (`core/src/providers/ollama.rs`) — `http://localhost:11434`
- **OpenAI-compatible** (`core/src/providers/openai_compatible.rs`) — any server speaking the
  OpenAI chat-completions shape: LM Studio, llama.cpp server, vLLM, LocalAI,
  text-generation-webui, or real OpenAI-compatible cloud APIs (auth via `apiKey`).

A new provider = a new implementation of `Provider` registered in
`core/src/providers/registry.rs`; nothing else needs to know about it.

### Dual tool-calling protocol (the reason small local models work)

Not every local model reliably supports native function calling. The agent loop handles this
by sending tool definitions via the provider's native tool-calling *and* teaching them to the
model as a plain-text fallback protocol in the system prompt — a fenced ` ```tool_call ` block
containing one JSON object (`{"name": ..., "arguments": {...}}`). While the model streams,
the loop scans visible text for that block (`core/src/agent/tool_call_parser.rs`) alongside
collecting any native tool calls the provider returns; native calls win if both are present,
otherwise the parsed fallback calls are used. Reasoning models that wrap thinking in
`<think>...</think>` are also filtered (`core/src/agent/thinking_filter.rs`): thinking is
pulled out of the visible stream and stripped before the text is stored back into history.

### The agent loop

`core/src/agent/mod.rs` is the turn loop, capped at 25 iterations: stream a completion,
filter thinking, resolve tool calls (native-or-fallback), run each tool via the shared tool
registry (`core/src/tools/registry.rs` — `read_file`, `write_file`, `edit_file`, `list_dir`,
`glob`, `grep`, `bash`), push a `tool` role message with the result, and repeat until the
model responds with plain text and no further tool calls. Tools flagged `mutating: true`
(writes, edits, bash) require approval before running unless auto-approve is on.

### Config

`~/.local-code/config.json`, with an optional project-local `.local-code.json` in the cwd
merged on top (project wins; providers are merged by `id`, keeping first-appearance order
but with later fields overriding). The desktop app additionally keeps its own projects/chats
state at `~/.local-code/desktop/workspace.json` (`desktop/app/src/workspace.rs`).

### Desktop app UI internals (`desktop/app/src`)

- `view.rs` — all layout; `theme.rs` — the only place colors/borders/shadows are defined,
  picked once at startup from the OS light/dark preference (not reactive mid-session) and
  applied everywhere via `theme::*` style functions rather than ad hoc styling in `view.rs`.
- `app_state.rs` — `State` + the `update()` message handler (iced's Elm-style architecture).
- `setup.rs` — the first-run/no-model-configured wizard: probes configured providers, falls
  back to checking for a local Ollama install (offers to start it, or pull a
  hardware-sized model recommendation), otherwise walks manual provider/model selection.
- The window is intentionally undecorated (`main.rs` sets `.decorations(false)`) in favor of
  a custom title bar (`view.rs::title_bar_view`) with its own drag-to-move and
  minimize/maximize/close. Losing OS decorations also loses OS-handled edge/corner resize,
  so that's reimplemented manually as invisible `mouse_area` regions calling
  `window::drag_resize` (see `window_chrome()` in `view.rs`). The window's `iced::window::Id`
  isn't known until the `Opened` event arrives via the `window::events()` subscription
  (captured into `State.window_id` in `app_state.rs`) — every window-control message handler
  guards on that being `Some` before issuing a `window::*` task.
