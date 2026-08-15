import { exec } from "node:child_process";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

const MAX_OUTPUT_CHARS = 20_000;
const TIMEOUT_MS = 120_000;

function truncate(text: string): string {
  if (text.length <= MAX_OUTPUT_CHARS) return text;
  return text.slice(0, MAX_OUTPUT_CHARS) + `\n... (truncated, ${text.length - MAX_OUTPUT_CHARS} more characters)`;
}

export const bashTool: Tool = {
  definition: {
    name: "bash",
    description:
      "Run a shell command in the project working directory and return its stdout/stderr. " +
      "Uses the OS default shell (PowerShell/cmd on Windows, sh elsewhere).",
    parameters: {
      type: "object",
      properties: {
        command: { type: "string", description: "The shell command to run." },
      },
      required: ["command"],
    },
    mutating: true,
  },

  preview(args) {
    return `$ ${String(args.command ?? "")}`;
  },

  async execute(args, ctx) {
    const command = String(args.command ?? "");
    if (!command) return err("Missing required argument: command");

    return new Promise((resolvePromise) => {
      exec(
        command,
        { cwd: ctx.cwd, timeout: TIMEOUT_MS, maxBuffer: 10 * 1024 * 1024 },
        (error, stdout, stderr) => {
          const combined = [stdout, stderr].filter(Boolean).join("\n").trim();
          const output = combined.length > 0 ? truncate(combined) : "(no output)";
          if (error) {
            resolvePromise(err(`${output}\n\nCommand exited with error: ${error.message}`));
          } else {
            resolvePromise(ok(output));
          }
        },
      );
    });
  },
};
