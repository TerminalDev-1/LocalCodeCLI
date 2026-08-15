import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { LocalCodeConfig, ProviderConfig } from "./types.js";

const CONFIG_DIR = join(homedir(), ".local-code");
const CONFIG_PATH = join(CONFIG_DIR, "config.json");
const PROJECT_CONFIG_PATH = join(process.cwd(), ".local-code.json");

const DEFAULT_CONFIG: LocalCodeConfig = {
  providers: [
    {
      id: "ollama",
      type: "ollama",
      baseUrl: "http://localhost:11434",
      label: "Ollama",
    },
    {
      id: "lmstudio",
      type: "openai-compatible",
      baseUrl: "http://localhost:1234/v1",
      label: "LM Studio",
    },
  ],
  defaultProvider: "ollama",
  defaultModel: "",
  autoApprove: false,
};

function readJson<T>(path: string): T | null {
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, "utf-8")) as T;
  } catch {
    return null;
  }
}

export function loadConfig(): LocalCodeConfig {
  const base = readJson<Partial<LocalCodeConfig>>(CONFIG_PATH) ?? {};
  const project = readJson<Partial<LocalCodeConfig>>(PROJECT_CONFIG_PATH) ?? {};

  const merged: LocalCodeConfig = {
    ...DEFAULT_CONFIG,
    ...base,
    ...project,
    providers: mergeProviders(DEFAULT_CONFIG.providers, base.providers, project.providers),
  };

  return merged;
}

function mergeProviders(
  ...lists: (ProviderConfig[] | undefined)[]
): ProviderConfig[] {
  const byId = new Map<string, ProviderConfig>();
  for (const list of lists) {
    for (const p of list ?? []) {
      byId.set(p.id, { ...byId.get(p.id), ...p });
    }
  }
  return [...byId.values()];
}

export function saveConfig(config: LocalCodeConfig): void {
  if (!existsSync(CONFIG_DIR)) mkdirSync(CONFIG_DIR, { recursive: true });
  writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2) + "\n", "utf-8");
}

export function getConfigPath(): string {
  return CONFIG_PATH;
}
