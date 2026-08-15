import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { createTwoFilesPatch } from "diff";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

export const writeFileTool: Tool = {
  definition: {
    name: "write_file",
    description:
      "Create a new file or overwrite an existing file with the given content. Creates parent directories as needed.",
    parameters: {
      type: "object",
      properties: {
        path: { type: "string", description: "File path, relative to the working directory or absolute." },
        content: { type: "string", description: "Full text content to write to the file." },
      },
      required: ["path", "content"],
    },
    mutating: true,
  },

  preview(args, ctx) {
    const path = String(args.path ?? "");
    const content = String(args.content ?? "");
    const fullPath = resolve(ctx.cwd, path);
    const before = existsSync(fullPath) ? readFileSync(fullPath, "utf-8") : "";
    if (before === content) return `No changes to ${path}`;
    return createTwoFilesPatch(path, path, before, content, "before", "after");
  },

  async execute(args, ctx) {
    const path = String(args.path ?? "");
    const content = args.content;
    if (!path) return err("Missing required argument: path");
    if (typeof content !== "string") return err("Missing required argument: content");

    const fullPath = resolve(ctx.cwd, path);
    try {
      mkdirSync(dirname(fullPath), { recursive: true });
      writeFileSync(fullPath, content, "utf-8");
    } catch (e) {
      return err(`Failed to write ${path}: ${(e as Error).message}`);
    }

    return ok(`Wrote ${content.split("\n").length} lines to ${path}`);
  },
};
