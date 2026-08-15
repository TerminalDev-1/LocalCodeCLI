import chalk from "chalk";

// Strips ANSI/SGR escape codes so we can pad boxed lines by their visible width, not byte length.
export function visibleLength(s: string): number {
  return s.replace(/\x1b\[[0-9;]*m/g, "").length;
}

// Terminal columns can be narrower than `min` (a split pane, an SSH session, a resized
// window). Previously boxWidth() floored to `min` unconditionally, so on a narrow
// terminal the box was drawn wider than the terminal itself — every row silently wrapped,
// which desyncs anything that redraws by moving the cursor up a fixed number of *logical*
// lines (see boxSelect.ts), corrupting/clipping the box. Never return a width that can't
// fit (plus the 2 border chars) in the actual terminal.
export function boxWidth(min = 40, max = 78): number {
  const cols = process.stdout.columns || 80;
  const available = Math.max(16, cols - 4);
  return Math.min(max, Math.max(Math.min(min, available), 16));
}

// Truncates already-styled (chalk-colored) text to fit within `width` visible columns.
// Truncation only happens for pathological over-length content, so dropping color codes
// in that case (rather than trying to splice around them) keeps this simple and correct.
function truncateVisible(s: string, width: number): string {
  if (width <= 0) return "";
  const stripped = s.replace(/\x1b\[[0-9;]*m/g, "");
  if (stripped.length <= width) return s;
  if (width === 1) return stripped.slice(0, 1);
  return stripped.slice(0, width - 1) + "…";
}

export function boxTop(width: number, title?: string): string {
  if (!title) return chalk.cyan("╭" + "─".repeat(width) + "╮");
  let label = ` ${title} `;
  if (label.length > width - 1) label = truncateVisible(label, Math.max(0, width - 1));
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
  if (visibleLength(content) > width) content = truncateVisible(content, width);
  const pad = Math.max(0, width - visibleLength(content));
  return chalk.cyan("│") + content + " ".repeat(pad) + chalk.cyan("│");
}

/** Left-pads a rendered line so it sits horizontally centered within the terminal width. */
export function centerLine(line: string): string {
  const cols = process.stdout.columns || 80;
  const pad = Math.max(0, Math.floor((cols - visibleLength(line)) / 2));
  return " ".repeat(pad) + line;
}

// Shared left-padding for a box of the given (unbordered) width, so the top/bottom border
// and the prompt line drawn between them — three separately-printed pieces — line up under
// the same center offset instead of each computing (and possibly disagreeing on) their own.
export function boxPad(width: number): string {
  const cols = process.stdout.columns || 80;
  return " ".repeat(Math.max(0, Math.floor((cols - width - 2) / 2)));
}
