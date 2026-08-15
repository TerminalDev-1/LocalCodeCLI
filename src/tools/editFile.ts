import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { createTwoFilesPatch } from "diff";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

function countOccurrences(haystack: string, needle: string): number {
  if (needle === "") return 0;
  let count = 0;
  let idx = 0;
  while ((idx = haystack.indexOf(needle, idx)) !== -1) {
    count += 1;
    idx += needle.length;
  }
  return count;
}

export const editFileTool: Tool = {
  definition: {
    name: "edit_file",
    description:
      "Replace an exact snippet of text in a file with new text. old_string must match exactly once in the file. " +
      "Use this for targeted edits instead of rewriting the whole file.",
    parameters: {
      type: "object",
      properties: {
        path: { type: "string", description: "File path, relative to the working directory or absolute." },
        old_string: { type: "string", description: "Exact text to find. Must be unique in the file." },
        new_string: { type: "string", description: "Text to replace it with." },
      },
      required: ["path", "old_string", "new_string"],
    },
    mutating: true,
  },

  preview(args, ctx) {
    const path = String(args.path ?? "");
    const oldString = String(args.old_string ?? "");
    const newString = String(args.new_string ?? "");
    const fullPath = resolve(ctx.cwd, path);
    if (!existsSync(fullPath)) return `File not found: ${path}`;
    const before = readFileSync(fullPath, "utf-8");
    const after = before.replace(oldString, newString);
    return createTwoFilesPatch(path, path, before, after, "before", "after");
  },

  async execute(args, ctx) {
    const path = String(args.path ?? "");
    const oldString = args.old_string;
    const newString = args.new_string;
    if (!path) return err("Missing required argument: path");
    if (typeof oldString !== "string") return err("Missing required argument: old_string");
    if (typeof newString !== "string") return err("Missing required argument: new_string");

    const fullPath = resolve(ctx.cwd, path);
    if (!existsSync(fullPath)) return err(`File not found: ${path}`);

    const before = readFileSync(fullPath, "utf-8");
    const occurrences = countOccurrences(before, oldString);
    if (occurrences === 0) {
      return err(`old_string not found in ${path}. Make sure it matches exactly, including whitespace.`);
    }
    if (occurrences > 1) {
      return err(
        `old_string matches ${occurrences} times in ${path}. Include more surrounding context so it matches exactly once.`,
      );
    }

    const after = before.replace(oldString, newString);
    try {
      writeFileSync(fullPath, after, "utf-8");
    } catch (e) {
      return err(`Failed to write ${path}: ${(e as Error).message}`);
    }

    return ok(`Edited ${path}`);
  },
};
