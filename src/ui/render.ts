import chalk from "chalk";

export function printBanner(providerLabel: string, model: string): void {
  console.log(chalk.bold.cyan("\n  Local Code") + chalk.dim("  — code with any model you point it at"));
  console.log(chalk.dim(`  provider: ${providerLabel}   model: ${model || "(none set)"}`));
  console.log(chalk.dim("  type your request, or /help for commands\n"));
}

export function printUserPrompt(): void {
  process.stdout.write(chalk.bold.green("you") + chalk.dim(" › ") );
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
