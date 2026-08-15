import type { ToolDefinition, ToolExecutionResult } from "../types.js";

export interface ToolContext {
  cwd: string;
}

export interface Tool {
  definition: ToolDefinition;
  execute(args: Record<string, unknown>, ctx: ToolContext): Promise<ToolExecutionResult>;
  /** Optional human-readable preview (e.g. a diff) shown before asking for approval. */
  preview?(args: Record<string, unknown>, ctx: ToolContext): Promise<string> | string;
}

export function ok(output: string): ToolExecutionResult {
  return { output, isError: false };
}

export function err(output: string): ToolExecutionResult {
  return { output, isError: true };
}
