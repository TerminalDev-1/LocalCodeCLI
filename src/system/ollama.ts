import { execFile, spawn } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export async function isOllamaInstalled(): Promise<boolean> {
  try {
    await execFileAsync("ollama", ["--version"]);
    return true;
  } catch {
    return false;
  }
}

export async function isOllamaReachable(baseUrl: string): Promise<boolean> {
  try {
    const res = await fetch(`${baseUrl}/api/tags`, { signal: AbortSignal.timeout(1500) });
    return res.ok;
  } catch {
    return false;
  }
}

/** Starts `ollama serve` detached from this process; caller should poll with waitForOllama. */
export function startOllamaServer(): void {
  const child = spawn("ollama", ["serve"], { detached: true, stdio: "ignore" });
  child.unref();
}

export async function waitForOllama(baseUrl: string, timeoutMs = 8000): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (await isOllamaReachable(baseUrl)) return true;
    await new Promise((r) => setTimeout(r, 400));
  }
  return false;
}

/** Runs `ollama pull <model>` with inherited stdio so Ollama's own progress bar shows through. */
export function pullModel(modelName: string): Promise<boolean> {
  return new Promise((resolve) => {
    const child = spawn("ollama", ["pull", modelName], { stdio: "inherit" });
    child.on("exit", (code) => resolve(code === 0));
    child.on("error", () => resolve(false));
  });
}
