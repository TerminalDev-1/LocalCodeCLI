import * as readline from "node:readline";
import * as readlinePromises from "node:readline/promises";
import chalk from "chalk";
import { boxBottom, boxDivider, boxLine, boxTop, boxWidth } from "./box.js";

export interface BoxChoice {
  title: string;
  value: string;
}

export interface BoxSelectOptions {
  title: string;
  choices: BoxChoice[];
  /** Enable type-to-filter (best for long lists like model names). */
  filterable?: boolean;
  initialIndex?: number;
}

const MAX_VISIBLE = 8;

/**
 * A boxed, arrow-key-driven list picker rendered in place (no scrollback spam).
 * Falls back to a plain numbered prompt when stdin/stdout isn't a real TTY
 * (piped input, non-interactive shells) since raw mode requires one.
 */
export async function boxSelect(opts: BoxSelectOptions): Promise<string | undefined> {
  if (opts.choices.length === 0) return undefined;
  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    return fallbackSelect(opts);
  }

  return new Promise((resolve) => {
    const width = boxWidth();
    let filter = "";
    let selected = Math.min(opts.initialIndex ?? 0, opts.choices.length - 1);
    let lastLineCount = 0;
    let settled = false;

    const visibleChoices = (): BoxChoice[] => {
      if (!opts.filterable || !filter) return opts.choices;
      const f = filter.toLowerCase();
      return opts.choices.filter((c) => c.title.toLowerCase().includes(f));
    };

    const render = () => {
      const items = visibleChoices();
      if (selected >= items.length) selected = Math.max(0, items.length - 1);

      const lines: string[] = [boxTop(width, opts.title)];

      if (opts.filterable) {
        lines.push(boxLine(` ${chalk.dim("search:")} ${filter}${chalk.inverse(" ")}`, width));
        lines.push(boxDivider(width));
      }

      let start = 0;
      if (items.length > MAX_VISIBLE) {
        start = Math.max(0, Math.min(selected - Math.floor(MAX_VISIBLE / 2), items.length - MAX_VISIBLE));
      }
      const slice = items.slice(start, start + MAX_VISIBLE);

      if (slice.length === 0) {
        lines.push(boxLine(`  ${chalk.dim("(no matches)")}`, width));
      }
      slice.forEach((item, i) => {
        const idx = start + i;
        const isSel = idx === selected;
        const marker = isSel ? chalk.green("›") : " ";
        const label = isSel ? chalk.bold.white(item.title) : chalk.gray(item.title);
        lines.push(boxLine(` ${marker} ${label}`, width));
      });
      if (items.length > MAX_VISIBLE) {
        lines.push(boxLine(chalk.dim(`  (${selected + 1}/${items.length})`), width));
      }

      lines.push(boxDivider(width));
      const footer = "↑↓ move · enter select · esc cancel" + (opts.filterable ? " · type to filter" : "");
      lines.push(boxLine(` ${chalk.dim(footer)}`, width));
      lines.push(boxBottom(width));

      if (lastLineCount > 0) {
        readline.moveCursor(process.stdout, 0, -lastLineCount);
        readline.cursorTo(process.stdout, 0);
        readline.clearScreenDown(process.stdout);
      }
      process.stdout.write(lines.join("\n") + "\n");
      lastLineCount = lines.length;
    };

    const cleanup = (result: string | undefined) => {
      if (settled) return;
      settled = true;
      process.stdin.removeListener("keypress", onKeypress);
      process.stdin.setRawMode(false);
      process.stdin.pause();

      readline.moveCursor(process.stdout, 0, -lastLineCount);
      readline.cursorTo(process.stdout, 0);
      readline.clearScreenDown(process.stdout);

      if (result !== undefined) {
        const chosen = opts.choices.find((c) => c.value === result);
        console.log(`  ${chalk.green("✓")} ${opts.title}: ${chalk.bold(chosen?.title ?? result)}`);
      } else {
        console.log(chalk.dim(`  ${opts.title}: cancelled`));
      }
      resolve(result);
    };

    const onKeypress = (str: string, key: readline.Key) => {
      const items = visibleChoices();
      if (key?.name === "up" || (key?.ctrl && key.name === "p")) {
        selected = Math.max(0, selected - 1);
        render();
      } else if (key?.name === "down" || (key?.ctrl && key.name === "n")) {
        selected = Math.min(items.length - 1, selected + 1);
        render();
      } else if (key?.name === "return") {
        cleanup(items[selected]?.value);
      } else if (key?.name === "escape" || (key?.ctrl && key.name === "c")) {
        cleanup(undefined);
      } else if (key?.name === "backspace" && opts.filterable) {
        filter = filter.slice(0, -1);
        selected = 0;
        render();
      } else if (opts.filterable && str && !key?.ctrl && !key?.meta && str.length === 1 && str >= " ") {
        filter += str;
        selected = 0;
        render();
      }
    };

    readline.emitKeypressEvents(process.stdin);
    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.on("keypress", onKeypress);
    render();
  });
}

// Shared across fallback prompts within one picker session. Consumed via the interface's
// async iterator rather than repeated .question() calls: .question() uses a single-slot
// callback, so if piped input arrives as one chunk with several lines already queued up,
// only the first line (the one active when the chunk is parsed) is delivered — the rest
// are emitted as 'line' events with nothing listening and are lost. The async iterator
// queues every line internally instead, so nothing is dropped regardless of timing.
// Callers must invoke closeFallbackSelectInterface() once the session is done.
let fallbackRl: readlinePromises.Interface | undefined;
let fallbackLines: AsyncIterator<string> | undefined;

function getFallbackLines(): AsyncIterator<string> {
  if (!fallbackRl) {
    fallbackRl = readlinePromises.createInterface({ input: process.stdin, output: process.stdout });
    fallbackLines = fallbackRl[Symbol.asyncIterator]();
  }
  return fallbackLines!;
}

export function closeFallbackSelectInterface(): void {
  fallbackRl?.close();
  fallbackRl = undefined;
  fallbackLines = undefined;
}

async function fallbackSelect(opts: BoxSelectOptions): Promise<string | undefined> {
  console.log(`\n${opts.title}`);
  opts.choices.forEach((c, i) => console.log(`  ${i + 1}) ${c.title}`));
  process.stdout.write("Enter a number: ");

  const { value, done } = await getFallbackLines().next();
  const answer = (done ? "" : value).trim();
  const idx = Number(answer) - 1;
  if (Number.isInteger(idx) && idx >= 0 && idx < opts.choices.length) {
    return opts.choices[idx]?.value;
  }
  const byValue = opts.choices.find((c) => c.value === answer || c.title === answer);
  return byValue?.value;
}
