import chalk from "chalk";
import prompts from "prompts";

let sessionAutoApprove = false;

function colorizeDiffLine(line: string): string {
  if (line.startsWith("+") && !line.startsWith("+++")) return chalk.green(line);
  if (line.startsWith("-") && !line.startsWith("---")) return chalk.red(line);
  if (line.startsWith("@@")) return chalk.cyan(line);
  return chalk.dim(line);
}

function renderPreview(preview: string): string {
  return preview
    .split("\n")
    .map((l) => "  " + colorizeDiffLine(l))
    .join("\n");
}

/**
 * Asks the user to approve a mutating tool call. Returns true if approved.
 * Honors a "always allow for this session" choice so the user isn't asked repeatedly.
 */
export async function confirmMutatingTool(name: string, preview: string): Promise<boolean> {
  if (sessionAutoApprove) return true;

  console.log(chalk.bold.yellow(`\n  ${name} wants to make changes:`));
  console.log(renderPreview(preview));

  const response = await prompts({
    type: "select",
    name: "choice",
    message: "Allow this?",
    choices: [
      { title: "Yes", value: "yes" },
      { title: "No", value: "no" },
      { title: "Yes, and don't ask again this session", value: "always" },
    ],
  });

  if (response.choice === "always") {
    sessionAutoApprove = true;
    return true;
  }
  return response.choice === "yes";
}
