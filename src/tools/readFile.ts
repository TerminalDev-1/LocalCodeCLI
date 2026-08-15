import { readFileSync, existsSync, statSync } from "node:fs";
import { resolve } from "node:path";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

const MAX_LINES = 2000;

export const readFileTool: Tool = {
  definition: {
    name: "read_file",
    description:
      "Read a text file from disk. Returns content with line numbers. Use offset/limit for large files.",
    parameters: {
      type: "object",
      properties: {
        path: { type: "string", description: "File path, relative to the working directory or absolute." },
        offset: { type: "number", description: "1-indexed line number to start reading from (optional)." },
        limit: { type: "number", description: "Maximum number of lines to read (optional, default 2000)." },
      },
      required: ["path"],
    },
    mutating: false,
  },

  async execute(args, ctx) {
    const path = String(args.path ?? "");
    if (!path) return err("Missing required argument: path");

    const fullPath = resolve(ctx.cwd, path);
    if (!existsSync(fullPath)) return err(`File not found: ${path}`);
    if (statSync(fullPath).isDirectory()) return err(`${path} is a directory, not a file.`);

    let raw: string;
    try {
      raw = readFileSync(fullPath, "utf-8");
    } catch (e) {
      return err(`Failed to read ${path}: ${(e as Error).message}`);
    }

    const lines = raw.split("\n");
    const offset = typeof args.offset === "number" && args.offset > 0 ? args.offset : 1;
    const limit = typeof args.limit === "number" && args.limit > 0 ? args.limit : MAX_LINES;
    const slice = lines.slice(offset - 1, offset - 1 + limit);

    const numbered = slice
      .map((line, i) => `${offset + i}\t${line}`)
      .join("\n");

    const truncated = offset - 1 + limit < lines.length;
    return ok(numbered + (truncated ? `\n... (${lines.length - (offset - 1 + limit)} more lines)` : ""));
  },
};
