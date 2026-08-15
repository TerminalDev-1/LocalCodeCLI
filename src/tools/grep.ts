import { readFileSync, statSync } from "node:fs";
import fg from "fast-glob";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

const MAX_FILE_BYTES = 1_000_000;
const MAX_MATCHES = 200;

export const grepTool: Tool = {
  definition: {
    name: "grep",
    description: "Search file contents for a regular expression. Returns matching file:line:text entries.",
    parameters: {
      type: "object",
      properties: {
        pattern: { type: "string", description: "Regular expression to search for." },
        path: { type: "string", description: "Directory or glob to restrict the search to (optional, defaults to the whole project)." },
      },
      required: ["pattern"],
    },
    mutating: false,
  },

  async execute(args, ctx) {
    const pattern = String(args.pattern ?? "");
    if (!pattern) return err("Missing required argument: pattern");

    let regex: RegExp;
    try {
      regex = new RegExp(pattern);
    } catch (e) {
      return err(`Invalid regular expression: ${(e as Error).message}`);
    }

    const globPattern = typeof args.path === "string" && args.path.length > 0 ? `${args.path.replace(/\/$/, "")}/**/*` : "**/*";

    let files: string[];
    try {
      files = await fg(globPattern, {
        cwd: ctx.cwd,
        ignore: ["**/node_modules/**", "**/.git/**", "**/dist/**"],
        onlyFiles: true,
        dot: false,
      });
    } catch (e) {
      return err(`Search failed: ${(e as Error).message}`);
    }

    const results: string[] = [];
    for (const file of files) {
      if (results.length >= MAX_MATCHES) break;
      const fullPath = `${ctx.cwd}/${file}`;
      try {
        if (statSync(fullPath).size > MAX_FILE_BYTES) continue;
        const content = readFileSync(fullPath, "utf-8");
        const lines = content.split("\n");
        for (let i = 0; i < lines.length; i++) {
          if (regex.test(lines[i] ?? "")) {
            results.push(`${file}:${i + 1}:${lines[i]}`);
            if (results.length >= MAX_MATCHES) break;
          }
        }
      } catch {
        continue; // binary or unreadable file, skip
      }
    }

    if (results.length === 0) return ok("(no matches)");
    const truncated = results.length >= MAX_MATCHES;
    return ok(results.join("\n") + (truncated ? `\n... (truncated at ${MAX_MATCHES} matches)` : ""));
  },
};
