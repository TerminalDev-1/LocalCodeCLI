import chalk from "chalk";
import prompts from "prompts";
import type { LocalCodeConfig, ProviderConfig } from "../types.js";
import { createProvider } from "../providers/registry.js";
import { getHardwareProfile, recommendModel } from "../system/hardware.js";
import { isOllamaInstalled, isOllamaReachable, pullModel, startOllamaServer, waitForOllama } from "../system/ollama.js";
import { boxSelect } from "./boxSelect.js";
import { printError, printNotice } from "./render.js";

// prompts calls onCancel (e.g. on Ctrl+C/Esc) instead of throwing; leaving it a no-op
// means the field just comes back undefined and callers treat that as "cancelled".
const onCancel = () => {};

const SKIP_VALUE = "__skip__";
const CUSTOM_VALUE = "__custom__";

/**
 * Boxed "select a provider" menu, with an escape hatch to register a new
 * OpenAI-compatible (or Ollama) endpoint on the fly — mirrors OpenCode's provider picker.
 */
export async function pickProvider(config: LocalCodeConfig): Promise<ProviderConfig | undefined> {
  const choices = config.providers.map((p) => ({
    title: `${p.label ?? p.id}  ${chalk.dim(p.baseUrl)}`,
    value: p.id,
  }));
  choices.push({ title: chalk.cyan("+ Add a custom provider..."), value: CUSTOM_VALUE });

  const providerId = await boxSelect({ title: "Select a provider", choices });

  if (!providerId) return undefined;
  if (providerId === CUSTOM_VALUE) return addCustomProvider(config);
  return config.providers.find((p) => p.id === providerId);
}

async function addCustomProvider(config: LocalCodeConfig): Promise<ProviderConfig | undefined> {
  const type = await boxSelect({
    title: "Provider type",
    choices: [
      { title: "OpenAI-compatible (OpenAI, LM Studio, vLLM, llama.cpp, LocalAI, ...)", value: "openai-compatible" },
      { title: "Ollama", value: "ollama" },
    ],
  });
  if (!type) return undefined;

  const answers = await prompts(
    [
      {
        type: "text",
        name: "id",
        message: "Provider id (short name, e.g. openai)",
        validate: (v: string) => (v.trim() ? true : "Required"),
      },
      {
        type: "text",
        name: "baseUrl",
        message: "Base URL",
        initial: type === "ollama" ? "http://localhost:11434" : "https://api.openai.com/v1",
        validate: (v: string) => (v.trim() ? true : "Required"),
      },
      { type: "text", name: "label", message: "Display label (optional)" },
      { type: "password", name: "apiKey", message: "API key (optional, leave blank if none)" },
    ],
    { onCancel },
  );

  if (!answers.id || !answers.baseUrl) return undefined;

  if (config.providers.some((p) => p.id === answers.id)) {
    printNotice(`Provider "${answers.id}" already exists — using the existing entry.`);
    return config.providers.find((p) => p.id === answers.id);
  }

  const newProvider: ProviderConfig = {
    id: answers.id,
    type: type as ProviderConfig["type"],
    baseUrl: answers.baseUrl,
    label: answers.label || answers.id,
    ...(answers.apiKey ? { apiKey: answers.apiKey } : {}),
  };

  config.providers.push(newProvider);
  return newProvider;
}

const MAX_UNFILTERED = 8;

/**
 * Boxed "select a model" menu. Queries the provider for its live model list
 * (filterable, for long lists); falls back to free-text entry if the provider
 * can't be reached or reports nothing.
 */
export async function pickModel(providerConfig: ProviderConfig): Promise<string | undefined> {
  const provider = createProvider(providerConfig);
  let models: string[] = [];
  try {
    models = await provider.listModels();
  } catch (e) {
    printError(`Could not reach ${provider.label}: ${(e as Error).message}`);
  }

  if (models.length === 0) {
    printNotice(`No models reported by ${provider.label} — enter one manually.`);
    const { model } = await prompts({ type: "text", name: "model", message: "Model name" }, { onCancel });
    return model || undefined;
  }

  return boxSelect({
    title: `Select a model (${models.length} available)`,
    choices: models.map((m) => ({ title: m, value: m })),
    filterable: models.length > MAX_UNFILTERED,
  });
}

export interface WizardResult {
  providerConfig: ProviderConfig;
  model: string;
}

async function hasAnyModelsAvailable(config: LocalCodeConfig): Promise<boolean> {
  const checks = await Promise.all(
    config.providers.map(async (p) => {
      try {
        const models = await Promise.race([
          createProvider(p).listModels(),
          new Promise<string[]>((resolve) => setTimeout(() => resolve([]), 2000)),
        ]);
        return models.length > 0;
      } catch {
        return false;
      }
    }),
  );
  return checks.some(Boolean);
}

/**
 * When nothing is reachable at all: check for a local Ollama install, start it if it's
 * just not running, size a recommendation off the machine's RAM, and offer a one-keypress
 * download of a small model so there's something to try the agent with immediately.
 */
async function tryAutoBootstrap(
  config: LocalCodeConfig,
  saveConfigFn: (config: LocalCodeConfig) => void,
): Promise<WizardResult | undefined> {
  printNotice("No models found on any configured provider. Checking for Ollama...");

  const ollamaProviderConfig: ProviderConfig =
    config.providers.find((p) => p.type === "ollama") ??
    { id: "ollama", type: "ollama", baseUrl: "http://localhost:11434", label: "Ollama" };

  // Check reachability first — this is the ground truth (it means Ollama is both
  // installed AND running). Only fall back to the CLI-based "is it installed at all"
  // check, which can false-negative on Windows if `ollama` isn't resolvable on PATH
  // from this process even though the app itself works fine.
  let reachable = await isOllamaReachable(ollamaProviderConfig.baseUrl);

  if (!reachable) {
    if (!(await isOllamaInstalled())) {
      printNotice("Ollama isn't installed. Install it from https://ollama.com/download and run local-code again — or set up a provider manually below.");
      return undefined;
    }

    printNotice("Ollama is installed but not running — starting it now...");
    await startOllamaServer();
    reachable = await waitForOllama(ollamaProviderConfig.baseUrl);
    if (!reachable) {
      printError("Couldn't reach Ollama after starting it. Try running `ollama serve` yourself and rerun local-code.");
      return undefined;
    }
  }

  const hw = getHardwareProfile();
  const { recommended, alternatives } = recommendModel(hw);

  console.log(
    chalk.dim(`\n  Your system: ~${Math.round(hw.totalRamGb)} GB RAM, ${hw.cpuCores} CPU cores.`) +
      chalk.dim(`\n  Recommended: `) +
      chalk.bold(recommended.name) +
      chalk.dim(` (~${recommended.approxSizeGb} GB) — ${recommended.description}\n`),
  );

  const choice = await boxSelect({
    title: "Download a model to get started?",
    choices: [
      { title: `${recommended.name}  ${chalk.dim(`(recommended, ~${recommended.approxSizeGb} GB)`)}`, value: recommended.name },
      ...alternatives.map((m) => ({ title: `${m.name}  ${chalk.dim(`(~${m.approxSizeGb} GB)`)}`, value: m.name })),
      { title: chalk.dim("Skip — I'll set this up manually"), value: SKIP_VALUE },
    ],
  });

  if (!choice || choice === SKIP_VALUE) return undefined;

  printNotice(`Pulling ${choice} via Ollama — this may take a few minutes depending on your connection...`);
  const ok = await pullModel(choice);
  if (!ok) {
    printError(`Failed to pull ${choice}. You can retry later with: ollama pull ${choice}`);
    return undefined;
  }

  if (!config.providers.some((p) => p.id === ollamaProviderConfig.id)) {
    config.providers.push(ollamaProviderConfig);
  }
  config.defaultProvider = ollamaProviderConfig.id;
  config.defaultModel = choice;
  saveConfigFn(config);
  printNotice(`${choice} is ready and saved as your default.`);

  return { providerConfig: ollamaProviderConfig, model: choice };
}

/**
 * First-run / no-model-configured flow: try to auto-bootstrap a small local model if
 * nothing at all is reachable, otherwise let the user pick a provider and model, and
 * offer to save both as the new default so they aren't dropped back into this every time.
 */
export async function runSetupWizard(
  config: LocalCodeConfig,
  saveConfigFn: (config: LocalCodeConfig) => void,
): Promise<WizardResult | undefined> {
  console.log(
    chalk.bold.cyan("\n  Welcome to Local Code!") +
      chalk.dim(" No model is configured yet — let's pick one.\n"),
  );

  if (!(await hasAnyModelsAvailable(config))) {
    const bootstrapped = await tryAutoBootstrap(config, saveConfigFn);
    if (bootstrapped) return bootstrapped;
    console.log(chalk.dim("\n  Let's set one up manually instead.\n"));
  }

  const providerConfig = await pickProvider(config);
  if (!providerConfig) return undefined;

  const model = await pickModel(providerConfig);
  if (!model) return undefined;

  const save = await boxSelect({
    title: "Save this as your default provider/model?",
    choices: [
      { title: "Yes", value: "yes" },
      { title: "No, just for this session", value: "no" },
    ],
  });

  if (save === "yes") {
    config.defaultProvider = providerConfig.id;
    config.defaultModel = model;
    saveConfigFn(config);
    printNotice(`Saved to config. Change it anytime with /provider or /model.`);
  }

  return { providerConfig, model };
}
