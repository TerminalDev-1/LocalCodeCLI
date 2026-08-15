// Best-effort heuristic for "does this model think out loud". There's no standard way to
// query a local model for this, so we pattern-match well-known reasoning/thinking model
// families by name. False negatives just mean the status line omits the badge; the model's
// <think> output (see agent/thinkingFilter.ts) is still rendered dimmed either way.
const REASONING_NAME_PATTERNS: RegExp[] = [
  /deepseek-?r1/i,
  /\bqwq\b/i,
  /qwen3/i,
  /magistral/i,
  /phi-4-reasoning/i,
  /\bo[134](-mini)?\b/i,
  /gpt-5-thinking/i,
  /glm-z1/i,
  /exaone-deep/i,
  /marco-o1/i,
  /\bcogito\b/i,
  /reasoning/i,
  /thinking/i,
];

/** Heuristically flags whether a model name looks like a reasoning/"thinking" model. */
export function isReasoningModel(model: string): boolean {
  if (!model) return false;
  return REASONING_NAME_PATTERNS.some((re) => re.test(model));
}
