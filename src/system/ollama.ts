import { execFile, spawn } from "node:child_process";
import { join } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

// On Windows, a plain `ollama` lookup can miss even when the app works fine from an
// interactive shell — PATH picked up by an installer often isn't visible to a process
// that was already running, and CreateProcess's PATH search doesn't always match what
// cmd.exe resolves. Fall back to the installer's default locations before giving up.
function ollamaCandidates(): string[] {
  const candidates = ["ollama"];
  if (process.platform === "win32") {
    if (process.env.LOCALAPPDATA) candidates.push(join(process.env.LOCALAPPDATA, "Programs", "Ollama", "ollama.exe"));
    if (process.env.ProgramFiles) candidates.push(join(process.env.ProgramFiles, "Ollama", "ollama.exe"));
  } else {
    candidates.push("/usr/local/bin/ollama", "/opt/homebrew/bin/ollama");
  }
  return candidates;
}

let resolvedOllamaPath: string | undefined;

async function resolveOllamaPath(): Promise<string | undefined> {
  if (resolvedOllamaPath) return resolvedOllamaPath;
  for (const candidate of ollamaCandidates()) {
    try {
      await execFileAsync(candidate, ["--version"], { shell: process.platform === "win32" });
      resolvedOllamaPath = candidate;
      return candidate;
    } catch {
      // try the next candidate
    }
  }
  return undefined;
}

export async function isOllamaInstalled(): Promise<boolean> {
  return (await resolveOllamaPath()) !== undefined;
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
export async function startOllamaServer(): Promise<void> {
  const ollamaPath = (await resolveOllamaPath()) ?? "ollama";
  const child = spawn(ollamaPath, ["serve"], {
    detached: true,
    stdio: "ignore",
    shell: process.platform === "win32",
  });
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
export async function pullModel(modelName: string): Promise<boolean> {
  const ollamaPath = (await resolveOllamaPath()) ?? "ollama";
  return new Promise((resolve) => {
    const child = spawn(ollamaPath, ["pull", modelName], {
      stdio: "inherit",
      shell: process.platform === "win32",
    });
    child.on("exit", (code) => resolve(code === 0));
    child.on("error", () => resolve(false));
  });
}
