//! Some local reasoning models (DeepSeek-R1, QwQ, Qwen3 thinking mode, ...) emit
//! their chain-of-thought inline as `<think>...</think>` before the real answer.
//! This strips those tags from the text stream, so raw markup never leaks into
//! the UI — callers can still choose to render "thinking" text dimmed.

use super::stream_util::floor_char_boundary;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub enum FilterEvent {
    Thinking(String),
    Text(String),
}

const OPEN: &str = "<think>";
const CLOSE: &str = "</think>";

#[derive(Default)]
pub struct ThinkingTagFilter {
    buffer: String,
    cursor: usize,
    in_thinking: bool,
}

impl ThinkingTagFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &str) -> Vec<FilterEvent> {
        self.buffer.push_str(chunk);
        self.drain(false)
    }

    pub fn finish(&mut self) -> Vec<FilterEvent> {
        self.drain(true)
    }

    fn drain(&mut self, final_: bool) -> Vec<FilterEvent> {
        let mut events = Vec::new();

        loop {
            let marker = if self.in_thinking { CLOSE } else { OPEN };
            let remaining = &self.buffer[self.cursor..];

            let Some(idx) = remaining.find(marker) else {
                let hold_back = if final_ { 0 } else { remaining.len().min(marker.len() - 1) };
                let emit_len = floor_char_boundary(remaining, remaining.len().saturating_sub(hold_back));
                if emit_len > 0 {
                    let text = remaining[..emit_len].to_string();
                    events.push(if self.in_thinking { FilterEvent::Thinking(text) } else { FilterEvent::Text(text) });
                    self.cursor += emit_len;
                }
                return events;
            };

            if idx > 0 {
                let text = remaining[..idx].to_string();
                events.push(if self.in_thinking { FilterEvent::Thinking(text) } else { FilterEvent::Text(text) });
            }
            self.cursor += idx + marker.len();
            self.in_thinking = !self.in_thinking;
        }
    }
}

static THINK_BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<think>.*?</think>").unwrap());

/// One-shot strip for text that's already fully accumulated (e.g. before saving to history).
pub fn strip_thinking_tags(text: &str) -> String {
    THINK_BLOCK.replace_all(text, "").trim().to_string()
}
