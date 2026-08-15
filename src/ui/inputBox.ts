import * as readline from "node:readline";
import * as readlinePromises from "node:readline/promises";
import chalk from "chalk";
import { boxBottom, boxLine, boxPad, boxTop, boxWidth } from "./box.js";

export interface BoxedLineOptions {
  title: string;
  /** Shared across calls so ↑/↓ can recall previous submissions, like a shell's history. */
  history: string[];
}

const BOX_ROWS = 3; // top border, content, bottom border
const CONTENT_ROW = 1;

function wordStart(s: string, from: number): number {
  let i = from;
  while (i > 0 && /\s/.test(s[i - 1]!)) i--;
  while (i > 0 && !/\s/.test(s[i - 1]!)) i--;
  return i;
}

/**
 * A single-line boxed text input, drawn and redrawn in place (raw mode + keypress, the
 * same approach boxSelect.ts uses) so the box has a real border on every side at all
 * times. Driving the box through node's readline instead — as the previous version did —
 * only ever gets you a bare, unbordered echoed line while typing, with the right border
 * missing until a hacky redraw closes it after Enter. This renders the whole box fresh on
 * every keystroke instead, so it never looks torn open.
 *
 * Falls back to a plain prompt when stdin/stdout isn't a real TTY (raw mode needs one).
 */
export async function readBoxedLine(opts: BoxedLineOptions): Promise<string | undefined> {
  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    return fallbackLine(opts.title);
  }

  return new Promise((resolve) => {
    const width = boxWidth();
    const pad = boxPad(width);
    const PREFIX_WIDTH = 3; // " ❯ " — visible width; chalk wraps it in ANSI codes that add to .length but not to what's drawn
    const prefix = " " + chalk.green("❯") + " ";
    const maxVisible = Math.max(1, width - PREFIX_WIDTH);

    let buf = "";
    let cursor = 0;
    let scrollOffset = 0;
    let historyIndex: number | null = null;
    let draft = "";
    let rendered = false;
    let settled = false;

    const render = () => {
      if (cursor - scrollOffset >= maxVisible) scrollOffset = cursor - maxVisible + 1;
      if (cursor < scrollOffset) scrollOffset = cursor;
      if (scrollOffset > buf.length) scrollOffset = Math.max(0, buf.length - maxVisible);

      const visible = buf.slice(scrollOffset, scrollOffset + maxVisible);
      const lines = [pad + boxTop(width, opts.title), pad + boxLine(prefix + visible, width), pad + boxBottom(width)];

      if (rendered) {
        readline.cursorTo(process.stdout, 0);
        readline.moveCursor(process.stdout, 0, -CONTENT_ROW);
        readline.clearScreenDown(process.stdout);
      }
      process.stdout.write(lines.join("\n") + "\n");
      rendered = true;

      const col = pad.length + 1 + PREFIX_WIDTH + (cursor - scrollOffset);
      readline.moveCursor(process.stdout, 0, -(BOX_ROWS - CONTENT_ROW));
      readline.cursorTo(process.stdout, col);
    };

    const finish = (value: string | undefined) => {
      if (settled) return;
      settled = true;
      process.stdin.removeListener("keypress", onKeypress);
      process.stdin.setRawMode(false);
      process.stdin.pause();
      readline.cursorTo(process.stdout, 0);
      readline.moveCursor(process.stdout, 0, BOX_ROWS - CONTENT_ROW);
      resolve(value);
    };

    const onKeypress = (str: string, key: readline.Key) => {
      if (settled) return;

      if (key?.ctrl && (key.name === "c" || key.name === "d")) {
        finish(undefined);
        return;
      }
      if (key?.name === "return") {
        finish(buf);
        return;
      }
      if (key?.name === "backspace") {
        if (cursor > 0) {
          buf = buf.slice(0, cursor - 1) + buf.slice(cursor);
          cursor--;
          render();
        }
        return;
      }
      if (key?.name === "delete") {
        if (cursor < buf.length) {
          buf = buf.slice(0, cursor) + buf.slice(cursor + 1);
          render();
        }
        return;
      }
      if (key?.ctrl && key.name === "w") {
        const start = wordStart(buf, cursor);
        buf = buf.slice(0, start) + buf.slice(cursor);
        cursor = start;
        render();
        return;
      }
      if (key?.ctrl && key.name === "u") {
        buf = buf.slice(cursor);
        cursor = 0;
        render();
        return;
      }
      if (key?.ctrl && key.name === "k") {
        buf = buf.slice(0, cursor);
        render();
        return;
      }
      if ((key?.ctrl && key.name === "a") || key?.name === "home") {
        cursor = 0;
        render();
        return;
      }
      if ((key?.ctrl && key.name === "e") || key?.name === "end") {
        cursor = buf.length;
        render();
        return;
      }
      if (key?.name === "left") {
        if (cursor > 0) {
          cursor--;
          render();
        }
        return;
      }
      if (key?.name === "right") {
        if (cursor < buf.length) {
          cursor++;
          render();
        }
        return;
      }
      if (key?.name === "up") {
        if (opts.history.length === 0) return;
        if (historyIndex === null) {
          draft = buf;
          historyIndex = opts.history.length - 1;
        } else if (historyIndex > 0) {
          historyIndex--;
        }
        buf = opts.history[historyIndex] ?? "";
        cursor = buf.length;
        render();
        return;
      }
      if (key?.name === "down") {
        if (historyIndex === null) return;
        if (historyIndex < opts.history.length - 1) {
          historyIndex++;
          buf = opts.history[historyIndex] ?? "";
        } else {
          historyIndex = null;
          buf = draft;
        }
        cursor = buf.length;
        render();
        return;
      }
      if (key?.name === "escape") {
        buf = "";
        cursor = 0;
        historyIndex = null;
        render();
        return;
      }

      // Plain character input — pasted chunks arrive as a multi-char `str` too.
      if (str && !key?.ctrl && !key?.meta) {
        const clean = str.replace(/[\r\n]/g, "");
        if (clean) {
          buf = buf.slice(0, cursor) + clean + buf.slice(cursor);
          cursor += clean.length;
          historyIndex = null;
          render();
        }
      }
    };

    readline.emitKeypressEvents(process.stdin);
    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.on("keypress", onKeypress);
    render();
  });
}

async function fallbackLine(title: string): Promise<string | undefined> {
  const rl = readlinePromises.createInterface({ input: process.stdin, output: process.stdout });
  process.stdout.write(`${title} > `);
  try {
    const { value, done } = await rl[Symbol.asyncIterator]().next();
    return done ? undefined : value;
  } finally {
    rl.close();
  }
}
