import os from "node:os";

export interface HardwareProfile {
  totalRamGb: number;
  freeRamGb: number;
  cpuCores: number;
}

export function getHardwareProfile(): HardwareProfile {
  return {
    totalRamGb: os.totalmem() / 1024 ** 3,
    freeRamGb: os.freemem() / 1024 ** 3,
    cpuCores: os.cpus().length,
  };
}

export interface ModelRecommendation {
  /** Name as passed to `ollama pull`. */
  name: string;
  approxSizeGb: number;
  description: string;
}

// All small enough to be a reasonable default download; picked by available RAM so the
// recommendation still fits comfortably rather than swapping to disk.
const TIERS: ModelRecommendation[] = [
  { name: "qwen2.5-coder:1.5b", approxSizeGb: 1.0, description: "tiny & fast — great for a first test on modest hardware" },
  { name: "qwen2.5-coder:3b", approxSizeGb: 1.9, description: "still small, noticeably better coding quality" },
  { name: "qwen2.5-coder:7b", approxSizeGb: 4.7, description: "the strongest of the small tiers, wants more RAM" },
];

export interface ModelRecommendationResult {
  recommended: ModelRecommendation;
  alternatives: ModelRecommendation[];
}

export function recommendModel(hw: HardwareProfile): ModelRecommendationResult {
  const idx = hw.totalRamGb >= 16 ? 2 : hw.totalRamGb >= 8 ? 1 : 0;
  return {
    recommended: TIERS[idx]!,
    alternatives: TIERS.filter((_, i) => i !== idx),
  };
}
