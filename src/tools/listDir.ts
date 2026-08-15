import { existsSync, readdirSync, statSync } from "node:fs";
import { resolve } from "node:path";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

export const listDirTool: Tool = {
  definition: {
    name: "list_dir",
    description: "List files and subdirectories at a given path (non-recursive).",
    parameters: {
      type: "object",
      properties: {
        path: { type: "string", description: "Directory path, relative to the working directory or absolute. Defaults to '.'." },
      },
      required: [],
    },
    mutating: false,
  },

  async execute(args, ctx) {
    const path = typeof args.path === "string" && args.path.length > 0 ? args.path : ".";
    const fullPath = resolve(ctx.cwd, path);
    if (!existsSync(fullPath)) return err(`Path not found: ${path}`);
    if (!statSync(fullPath).isDirectory()) return err(`${path} is not a directory.`);

    const entries = readdirSync(fullPath, { withFileTypes: true })
      .filter((e) => e.name !== "node_modules" && e.name !== ".git")
      .sort((a, b) => a.name.localeCompare(b.name));

    const lines = entries.map((e) => (e.isDirectory() ? `${e.name}/` : e.name));
    return ok(lines.length > 0 ? lines.join("\n") : "(empty directory)");
  },
};
