import chalk from "chalk";
import { centerLine } from "./box.js";
import { renderLogo } from "./logo.js";
import { isReasoningModel } from "../system/reasoning.js";

// No placeholder input box here — the real "you" box (drawn by the REPL's input loop,
// see repl.ts) always appears immediately below this banner, so a second decorative one
// was just dead space stacked on top of the one you actually type into.
export function printBanner(providerLabel: string, model: string): void {
  const modelText = model ? chalk.bold(model) : chalk.red("(none set — try /model)");

  console.log();
  for (const line of renderLogo().split("\n")) console.log(centerLine(line));
  console.log(centerLine(chalk.dim("code with any model you point it at — local or cloud")));
  console.log();

  const status = [`${chalk.dim("provider")} ${chalk.bold(providerLabel)}`, `${chalk.dim("model")} ${modelText}`];
  if (model && isReasoningModel(model)) status.push(chalk.magenta("◆ reasoning"));
  console.log(centerLine(status.join(chalk.dim("   ·   "))));
  console.log(centerLine(chalk.dim("/help for commands   /model, /provider to switch")));
  console.log();
}

export function printAssistantLabel(): void {
  process.stdout.write(chalk.bold.magenta("local-code") + chalk.dim(" › "));
}

export function writeAssistantText(text: string): void {
  process.stdout.write(text);
}

export function writeThinkingText(text: string): void {
  process.stdout.write(chalk.dim.italic(text));
}

export function printToolStart(name: string, args: Record<string, unknown>): void {
  const argsPreview = summarizeArgs(args);
  console.log(chalk.dim(`  → ${name}(${argsPreview})`));
}

function summarizeArgs(args: Record<string, unknown>): string {
  return Object.entries(args)
    .map(([k, v]) => {
      const s = typeof v === "string" ? v : JSON.stringify(v);
      const trimmed = s.length > 60 ? s.slice(0, 57) + "..." : s;
      return `${k}: ${trimmed}`;
    })
    .join(", ");
}

export function printToolResult(name: string, output: string, isError: boolean): void {
  const lines = output.split("\n");
  const preview = lines.slice(0, 8).join("\n");
  const truncated = lines.length > 8 ? `\n  ${chalk.dim(`... (${lines.length - 8} more lines)`)}` : "";
  const color = isError ? chalk.red : chalk.dim;
  const indented = preview
    .split("\n")
    .map((l) => `  ${l}`)
    .join("\n");
  console.log(color(indented) + truncated);
}

export function printNotice(message: string): void {
  console.log(chalk.yellow(`  ! ${message}`));
}

export function printError(message: string): void {
  console.log(chalk.red(`  ✗ ${message}`));
}

export function printSuccess(message: string): void {
  console.log(chalk.green(`  ${message}`));
}

export function newline(): void {
  console.log();
}
