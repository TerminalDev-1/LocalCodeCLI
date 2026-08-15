# Local Code Desktop

A native Windows desktop app for Local Code, built with Rust and [iced](https://iced.rs).
Same agent loop and provider setup as the CLI in the repo root — this is a project/chat
workspace UI on top of it, styled after Cursor's composer-centric layout.

## Layout

```
desktop/
  core/    local_code_core — providers, tools, the agent loop itself (UI-agnostic)
  app/     local_code_desktop — the iced application: view, state, theme, window chrome
```

- `core/src/providers/` — Ollama + OpenAI-compatible streaming adapters
- `core/src/tools/` — read_file, write_file, edit_file, list_dir, glob, grep, bash
- `core/src/agent/` — the agent loop, system prompt, tool-call parsing, thinking-block filtering
- `app/src/view.rs` — all UI layout (sidebar, composer, transcript, custom window chrome)
- `app/src/theme.rs` — the light/dark palette and widget styles
- `app/src/app_state.rs` — application state and the `update()` message handler
- `app/src/setup.rs` — first-run/no-model-configured provider + model picker wizard
- `app/src/workspace.rs` — projects/chats persistence (separate from CLI config, see below)

## Build & run

Requires a Rust toolchain (`rustup`).

```bash
cd desktop
cargo build -p local_code_desktop
cargo run -p local_code_desktop
```

The window is undecorated with a custom title bar (drag to move, edge/corner regions to
resize, minimize/maximize/close in the top-right) instead of the OS chrome.

## Configuration

Provider/model config is shared with the CLI at `~/.local-code/config.json`. Projects and
chats are the desktop app's own state, stored separately at
`~/.local-code/desktop/workspace.json`, since they don't apply to the CLI.

If no default model is configured, launching the app starts the same setup flow as the CLI:
probe configured providers, fall back to checking for a local Ollama install, and offer to
pull a model sized to your hardware if nothing is set up yet.
