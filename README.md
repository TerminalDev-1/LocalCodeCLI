# Local Code

A terminal coding agent, built from scratch, that works with any model you point it at —
local or cloud. It reads files, edits them, runs shell commands, and iterates in a loop
until the task is done, same shape as tools like Claude Code or OpenCode, but provider-agnostic
from the ground up.

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
QwQ, Qwen thinking mode) are also handled — their thinking is dimmed in the terminal and
stripped from the conversation history sent back to the model.

## Install

```bash
npm install
npm run build
npm link
```

`npm link` puts `local-code` on your PATH. Alternatively run it directly with
`npm run dev` (via `tsx`, no build step) or `node dist/index.js` after building.

## Quick start

Start [Ollama](https://ollama.com) or [LM Studio](https://lmstudio.ai) and load a model, then:

```bash
local-code models                 # see what's available from each provider
local-code config set-provider ollama
local-code config set-model llama3.1:8b
local-code                        # start an interactive session in the current directory
```

Or skip config and pass flags directly:

```bash
local-code --provider lmstudio --model qwen2.5-coder-7b "add input validation to src/api.ts"
```

## Usage

```
local-code [options] [prompt]

Arguments:
  prompt              initial message to send; omit to start an interactive session

Options:
  --provider <id>      provider id to use
  -m, --model <name>   model name to use
  -y, --yolo           auto-approve tool calls without asking
  --print              run one turn non-interactively, print the result, and exit

Commands:
  models [--provider <id>]   list models available from configured providers
  config                     show the resolved config and its file path
  config set-model <name>    set the default model
  config set-provider <id>   set the default provider
```

In-session slash commands: `/help`, `/model <name>`, `/provider <id>`, `/clear`, `/exit`.

File edits and shell commands are previewed (as a diff, or the literal command) and require
approval before running, unless you pass `-y/--yolo` or choose "don't ask again" in-session.
When stdin isn't a TTY (e.g. piped input, `--print` in a script), approval is skipped
automatically to avoid hanging.

## Configuration

Config lives at `~/.local-code/config.json`, with an optional per-project override at
`.local-code.json` in the current directory (both are merged, project wins). Shape:

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

## Tools available to the agent

`read_file`, `write_file`, `edit_file` (exact string replace), `list_dir`, `glob`, `grep`, `bash`.

## Project layout

```
src/
  types.ts             shared types (messages, tool calls, provider interface)
  config.ts            load/save ~/.local-code/config.json + project overrides
  providers/           Ollama + OpenAI-compatible streaming adapters
  tools/                read/write/edit/list/glob/grep/bash implementations
  agent/
    systemPrompt.ts     builds the system prompt + tool docs
    toolCallParser.ts   streaming scanner for the ```tool_call fallback protocol
    thinkingFilter.ts   strips/dims <think> reasoning blocks
    loop.ts             the agent loop: call model, run tools, repeat until done
  ui/                   terminal rendering + approval prompts
  repl.ts, cli.ts, index.ts   interactive session + commander CLI wiring
```
