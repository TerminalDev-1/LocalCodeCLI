import fg from "fast-glob";
import type { Tool } from "./types.js";
import { ok, err } from "./types.js";

export const globTool: Tool = {
  definition: {
    name: "glob",
    description: "Find files matching a glob pattern, e.g. 'src/**/*.ts'. Returns matching paths.",
    parameters: {
      type: "object",
      properties: {
        pattern: { type: "string", description: "Glob pattern to match files against." },
      },
      required: ["pattern"],
    },
    mutating: false,
  },

  async execute(args, ctx) {
    const pattern = String(args.pattern ?? "");
    if (!pattern) return err("Missing required argument: pattern");

    try {
      const matches = await fg(pattern, {
        cwd: ctx.cwd,
        ignore: ["**/node_modules/**", "**/.git/**"],
        dot: false,
        onlyFiles: true,
      });
      matches.sort();
      return ok(matches.length > 0 ? matches.join("\n") : "(no matches)");
    } catch (e) {
      return err(`Glob failed: ${(e as Error).message}`);
    }
  },
};
