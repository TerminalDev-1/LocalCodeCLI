import * as readline from "node:readline/promises";
import chalk from "chalk";
import type { LocalCodeConfig, Message } from "./types.js";
import { resolveProvider } from "./providers/registry.js";
import { allTools } from "./tools/registry.js";
import { buildSystemPrompt } from "./agent/systemPrompt.js";
import { runAgentTurn } from "./agent/loop.js";
import { confirmMutatingTool } from "./ui/confirm.js";
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
  /help            show this help
  /model <name>    switch model for this session
  /clear           clear conversation history
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

  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  rl.on("SIGINT", () => {
    newline();
    rl.close();
  });

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
            return confirmMutatingTool(name, preview);
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

  while (true) {
    let line: string;
    try {
      line = await rl.question(chalk.bold.green("you") + chalk.dim(" › "));
    } catch {
      break; // stream closed (Ctrl+C / Ctrl+D)
    }

    const trimmed = line.trim();
    if (!trimmed) continue;

    if (trimmed === "/exit" || trimmed === "/quit") break;

    if (trimmed === "/help") {
      console.log(HELP_TEXT);
      continue;
    }

    if (trimmed === "/clear") {
      messages = [{ role: "system", content: buildSystemPrompt(toolDefs, opts.cwd) }];
      printNotice("Conversation cleared.");
      continue;
    }

    if (trimmed.startsWith("/model ")) {
      model = trimmed.slice("/model ".length).trim();
      printNotice(`Switched to model: ${model}`);
      continue;
    }

    if (trimmed.startsWith("/provider ")) {
      providerId = trimmed.slice("/provider ".length).trim();
      try {
        provider = resolveProvider(opts.config, providerId);
        printNotice(`Switched to provider: ${provider.label}`);
      } catch (e) {
        printError((e as Error).message);
      }
      continue;
    }

    if (trimmed.startsWith("/")) {
      printError(`Unknown command: ${trimmed}. Type /help for a list.`);
      continue;
    }

    await runTurn(trimmed);
  }

  rl.close();
}
