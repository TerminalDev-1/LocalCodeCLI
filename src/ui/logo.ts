import chalk from "chalk";

// Tiny 3x5 block font — just enough letters to spell LOCAL CODE. '#' = filled pixel.
const GLYPHS: Record<string, string[]> = {
  L: ["#  ", "#  ", "#  ", "#  ", "###"],
  O: [" # ", "# #", "# #", "# #", " # "],
  C: [" ##", "#  ", "#  ", "#  ", " ##"],
  A: [" # ", "# #", "###", "# #", "# #"],
  D: ["## ", "# #", "# #", "# #", "## "],
  E: ["###", "#  ", "## ", "#  ", "###"],
};

function renderWord(word: string): string[] {
  const rows = ["", "", "", "", ""];
  for (const ch of word) {
    const glyph = GLYPHS[ch];
    if (!glyph) continue;
    for (let r = 0; r < 5; r++) rows[r] += glyph[r] + " ";
  }
  return rows;
}

function lerp(a: number, b: number, t: number): number {
  return Math.round(a + (b - a) * t);
}

// Colors each '#' pixel with a horizontal gradient; everything else renders as plain space.
// Pixels render two characters wide (vs. one tall) so the glyphs read closer to square
// instead of squashed, since terminal cells are taller than they are wide.
function gradientize(rows: string[], from: [number, number, number], to: [number, number, number]): string[] {
  const width = Math.max(...rows.map((r) => r.length));
  return rows.map((row) => {
    let out = "";
    for (let i = 0; i < row.length; i++) {
      if (row[i] !== "#") {
        out += "  ";
        continue;
      }
      const t = width <= 1 ? 0 : i / (width - 1);
      const r = lerp(from[0], to[0], t);
      const g = lerp(from[1], to[1], t);
      const b = lerp(from[2], to[2], t);
      out += chalk.rgb(r, g, b)("██");
    }
    return out;
  });
}

export function renderLogo(): string {
  const left = renderWord("LOCAL");
  const right = renderWord("CODE");
  const combined = left.map((row, i) => row + "   " + right[i]);
  return gradientize(combined, [56, 189, 248], [232, 121, 249]).join("\n");
}
