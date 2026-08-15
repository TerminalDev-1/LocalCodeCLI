import * as readline from "node:readline/promises";
import chalk from "chalk";
import type { LocalCodeConfig, Message } from "./types.js";
import { resolveProvider } from "./providers/registry.js";
import { allTools } from "./tools/registry.js";
import { buildSystemPrompt } from "./agent/systemPrompt.js";
import { runAgentTurn } from "./agent/loop.js";
import { confirmMutatingTool as confirmMutatingToolUI } from "./ui/confirm.js";
import { pickModel, pickProvider, runSetupWizard } from "./ui/setup.js";
import { saveConfig } from "./config.js";
import { closeFallbackSelectInterface } from "./ui/boxSelect.js";
import { boxBottom, boxTop, boxWidth } from "./ui/box.js";
import {
  printAssistantLabel,
  printBanner,
  printError,
  printNotice,
  printToolResult,
  printToolStart,
  writeAssistantText,
  writeThinkingText,
  newline,
} from "./ui/render.js";

const HELP_TEXT = `Commands:
  /model           pick a model for the current provider (interactive)
  /model <name>    switch model directly, by name
  /provider        pick a provider (interactive)
  /provider <id>   switch provider directly, by id
  /models          list models available from the current provider
  /onboarding      rerun the first-run setup wizard (hardware check + model picker)
  /clear           clear conversation history
  /help            show this help
  /exit, /quit     exit`;

export interface ReplOptions {
  config: LocalCodeConfig;
  providerId: string;
  model: string;
  autoApprove: boolean;
  cwd: string;
  initialPrompt?: string;
  printOnly?: boolean;
}

export async function startRepl(opts: ReplOptions): Promise<void> {
  let providerId = opts.providerId;
  let model = opts.model;
  let provider = resolveProvider(opts.config, providerId);

  const toolDefs = allTools.map((t) => t.definition);
  let messages: Message[] = [{ role: "system", content: buildSystemPrompt(toolDefs, opts.cwd) }];

  if (!opts.printOnly) printBanner(provider.label, model);

  const newReadline = (): readline.Interface => {
    const iface = readline.createInterface({ input: process.stdin, output: process.stdout });
    iface.on("SIGINT", () => {
      newline();
      iface.close();
    });
    return iface;
  };

  let rl = newReadline();
  // Read lines via the async iterator rather than repeated rl.question() calls:
  // .question() captures exactly one line per call through a single-slot callback, so
  // when piped input arrives as one chunk with several lines already queued (e.g. a
  // scripted multi-turn conversation), every line past the first is parsed, emitted,
  // and dropped before the next question() re-arms. The iterator queues them properly.
  let lines = rl[Symbol.asyncIterator]();

  // The boxed model/provider pickers and tool-approval prompt drive stdin themselves
  // (raw mode + keypress events) to draw their box — that would collide with this
  // readline interface's own keypress handling if both were listening at once. Close
  // it for the duration of the sub-prompt, then reopen a fresh one afterward.
  const withPausedRl = async <T>(fn: () => Promise<T>): Promise<T> => {
    rl.close();
    try {
      return await fn();
    } finally {
      closeFallbackSelectInterface();
      rl = newReadline();
      lines = rl[Symbol.asyncIterator]();
    }
  };

  const runTurn = async (userText: string) => {
    messages.push({ role: "user", content: userText });
    let labelPrinted = false;

    try {
      await runAgentTurn({
        provider,
        model,
        messages,
        cwd: opts.cwd,
        autoApprove: opts.autoApprove,
        callbacks: {
          onTextChunk(text) {
            if (!labelPrinted) {
              printAssistantLabel();
              labelPrinted = true;
            }
            writeAssistantText(text);
          },
          onThinkingChunk(text) {
            if (!labelPrinted) {
              printAssistantLabel();
              labelPrinted = true;
            }
            writeThinkingText(text);
          },
          onToolStart(name, args) {
            if (labelPrinted) newline();
            printToolStart(name, args);
          },
          async confirmMutatingTool(name, preview) {
            return withPausedRl(() => confirmMutatingToolUI(name, preview));
          },
          onToolResult(name, result) {
            printToolResult(name, result.output, result.isError);
          },
          onNotice(message) {
            printNotice(message);
          },
        },
      });
    } catch (e) {
      printError((e as Error).message);
    }

    newline();
    newline();
  };

  if (opts.initialPrompt) {
    await runTurn(opts.initialPrompt);
  }

  if (opts.printOnly) {
    rl.close();
    return;
  }

  // The boxed input framing relies on the terminal echoing the typed line and its
  // trailing Enter as a real newline before we draw the bottom border. Piped/non-TTY
  // stdin isn't echoed, so the border would land glued onto the same line as the
  // prompt — fall back to the plain single-line prompt there instead.
  const boxedInput = Boolean(process.stdout.isTTY);

  while (true) {
    const width = boxWidth();
    if (boxedInput) console.log(boxTop(width, "you"));

    const promptText = boxedInput
      ? chalk.cyan("│") + " " + chalk.green("❯") + " "
      : chalk.bold.green("you") + chalk.dim(" › ");
    // Piped stdin can hit EOF (auto-closing rl) before we've drained lines the
    // iterator already had buffered from an earlier chunk — skip the prompt write in
    // that case rather than throwing, and still try to read out what's left below.
    try {
      rl.setPrompt(promptText);
      rl.prompt();
    } catch {
      // rl already closed; fall through to draining any buffered lines below.
    }

    let line: string;
    try {
      const next = await lines.next();
      if (next.done) break; // stream closed (Ctrl+C / Ctrl+D)
      line = next.value;
    } catch {
      break;
    }
    if (boxedInput) console.log(boxBottom(width));

    const trimmed = line.trim();
    if (!trimmed) continue;

    if (trimmed.startsWith("/")) {
      const spaceIdx = trimmed.indexOf(" ");
      const cmd = spaceIdx === -1 ? trimmed : trimmed.slice(0, spaceIdx);
      const arg = spaceIdx === -1 ? "" : trimmed.slice(spaceIdx + 1).trim();

      switch (cmd) {
        case "/exit":
        case "/quit":
          rl.close();
          return;

        case "/help":
          console.log(HELP_TEXT);
          break;

        case "/clear":
          messages = [{ role: "system", content: buildSystemPrompt(toolDefs, opts.cwd) }];
          printNotice("Conversation cleared.");
          break;

        case "/model": {
          if (arg) {
            model = arg;
            printNotice(`Switched to model: ${model}`);
            break;
          }
          const providerConfig = opts.config.providers.find((p) => p.id === providerId);
          if (!providerConfig) {
            printError(`Current provider "${providerId}" isn't in the config.`);
            break;
          }
          const picked = await withPausedRl(() => pickModel(providerConfig));
          if (picked) {
            model = picked;
          }
          break;
        }

        case "/provider": {
          if (arg) {
            providerId = arg;
            try {
              provider = resolveProvider(opts.config, providerId);
              printNotice(`Switched to provider: ${provider.label}`);
            } catch (e) {
              printError((e as Error).message);
            }
            break;
          }
          const providerConfig = await withPausedRl(() => pickProvider(opts.config));
          if (!providerConfig) break;
          providerId = providerConfig.id;
          provider = resolveProvider(opts.config, providerId);
          const pickedModel = await withPausedRl(() => pickModel(providerConfig));
          if (pickedModel) {
            model = pickedModel;
          }
          break;
        }

        case "/onboarding": {
          const picked = await withPausedRl(() => runSetupWizard(opts.config, saveConfig));
          if (picked) {
            providerId = picked.providerConfig.id;
            provider = resolveProvider(opts.config, providerId);
            model = picked.model;
            printNotice(`Switched to ${provider.label} / ${model}`);
          }
          break;
        }

        case "/models": {
          const providerConfig = opts.config.providers.find((p) => p.id === providerId);
          if (!providerConfig) {
            printError(`Current provider "${providerId}" isn't in the config.`);
            break;
          }
          try {
            const models = await provider.listModels();
            if (models.length === 0) {
              printNotice(`${provider.label} reported no models — is it running?`);
            } else {
              for (const m of models) {
                const marker = m === model ? chalk.green("● ") : "  ";
                console.log(`  ${marker}${m}`);
              }
            }
          } catch (e) {
            printError((e as Error).message);
          }
          break;
        }

        default:
          printError(`Unknown command: ${cmd}. Type /help for a list.`);
      }

      continue;
    }

    await runTurn(trimmed);
  }

  rl.close();
}
