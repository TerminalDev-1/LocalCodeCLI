import chalk from "chalk";

// Strips ANSI/SGR escape codes so we can pad boxed lines by their visible width, not byte length.
export function visibleLength(s: string): number {
  return s.replace(/\x1b\[[0-9;]*m/g, "").length;
}

export function boxWidth(min = 40, max = 78): number {
  const cols = process.stdout.columns || 80;
  return Math.max(min, Math.min(max, cols - 4));
}

export function boxTop(width: number, title?: string): string {
  if (!title) return chalk.cyan("╭" + "─".repeat(width) + "╮");
  const label = ` ${title} `;
  const rest = Math.max(0, width - label.length - 1);
  return chalk.cyan("╭─" + chalk.bold(label) + "─".repeat(rest) + "╮");
}

export function boxBottom(width: number): string {
  return chalk.cyan("╰" + "─".repeat(width) + "╯");
}

export function boxDivider(width: number): string {
  return chalk.cyan("├" + "─".repeat(width) + "┤");
}

export function boxLine(content: string, width: number): string {
  const pad = Math.max(0, width - visibleLength(content));
  return chalk.cyan("│") + content + " ".repeat(pad) + chalk.cyan("│");
}
