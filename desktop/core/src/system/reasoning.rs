//! Best-effort heuristic for "does this model think out loud". There's no standard way to
//! query a local model for this, so we pattern-match well-known reasoning/thinking model
//! families by name. False negatives just mean the status line omits the badge; the model's
//! `<think>` output (see `agent::thinking_filter`) is still rendered dimmed either way.

use regex::Regex;
use std::sync::LazyLock;

static REASONING_NAME_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)deepseek-?r1",
        r"(?i)\bqwq\b",
        r"(?i)qwen3",
        r"(?i)magistral",
        r"(?i)phi-4-reasoning",
        r"(?i)\bo[134](-mini)?\b",
        r"(?i)gpt-5-thinking",
        r"(?i)glm-z1",
        r"(?i)exaone-deep",
        r"(?i)marco-o1",
        r"(?i)\bcogito\b",
        r"(?i)reasoning",
        r"(?i)thinking",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

/// Heuristically flags whether a model name looks like a reasoning/"thinking" model.
pub fn is_reasoning_model(model: &str) -> bool {
    if model.is_empty() {
        return false;
    }
    REASONING_NAME_PATTERNS.iter().any(|re| re.is_match(model))
}
