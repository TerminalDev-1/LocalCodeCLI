import type { LocalCodeConfig, Provider, ProviderConfig } from "../types.js";
import { createOllamaProvider } from "./ollama.js";
import { createOpenAiCompatibleProvider } from "./openaiCompatible.js";

export function createProvider(config: ProviderConfig): Provider {
  switch (config.type) {
    case "ollama":
      return createOllamaProvider(config);
    case "openai-compatible":
      return createOpenAiCompatibleProvider(config);
    default: {
      const exhaustive: never = config.type;
      throw new Error(`Unknown provider type: ${exhaustive}`);
    }
  }
}

export function resolveProvider(
  config: LocalCodeConfig,
  providerId?: string,
): Provider {
  const id = providerId ?? config.defaultProvider;
  const providerConfig = config.providers.find((p) => p.id === id);
  if (!providerConfig) {
    const known = config.providers.map((p) => p.id).join(", ");
    throw new Error(`Unknown provider "${id}". Configured providers: ${known}`);
  }
  return createProvider(providerConfig);
}

export function listProviders(config: LocalCodeConfig): ProviderConfig[] {
  return config.providers;
}
