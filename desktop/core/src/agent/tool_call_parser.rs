//! Universal, provider-agnostic tool-calling protocol.
//!
//! Not every local model supports native function calling reliably (especially
//! small ones), so Local Code also understands a plain-text protocol: a fenced
//! code block tagged `tool_call` containing a single JSON object. This works
//! with any model that can follow instructions and emit a code block, and it
//! is scanned for incrementally as text streams in so the UI can render
//! prose normally while hiding the raw JSON.

use super::stream_util::floor_char_boundary;
use crate::id::generate_id;
use crate::types::ToolCall;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub name: String,
    pub arguments: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Text(String),
    ToolCall { call: Option<ParsedToolCall>, raw: String },
}

const OPEN: &str = "```tool_call";
const CLOSE: &str = "```";

fn safe_parse(content: &str) -> Option<ParsedToolCall> {
    let parsed: Value = serde_json::from_str(content).ok()?;
    let name = parsed.get("name")?.as_str()?.to_string();
    let arguments = parsed.get("arguments").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    Some(ParsedToolCall { name, arguments })
}

/// Incrementally scans streamed text for `tool_call` fenced blocks.
#[derive(Default)]
pub struct StreamingToolCallScanner {
    buffer: String,
    cursor: usize,
}

impl StreamingToolCallScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &str) -> Vec<ScanEvent> {
        self.buffer.push_str(chunk);
        self.drain(false)
    }

    pub fn finish(&mut self) -> Vec<ScanEvent> {
        self.drain(true)
    }

    fn drain(&mut self, final_: bool) -> Vec<ScanEvent> {
        let mut events = Vec::new();

        loop {
            let remaining = &self.buffer[self.cursor..];

            let Some(open_idx) = remaining.find(OPEN) else {
                // Hold back a small margin in case the fence marker is split across chunks.
                let hold_back = if final_ { 0 } else { remaining.len().min(OPEN.len() - 1) };
                let emit_len = floor_char_boundary(remaining, remaining.len().saturating_sub(hold_back));
                if emit_len > 0 {
                    events.push(ScanEvent::Text(remaining[..emit_len].to_string()));
                    self.cursor += emit_len;
                }
                break;
            };

            if open_idx > 0 {
                events.push(ScanEvent::Text(remaining[..open_idx].to_string()));
                self.cursor += open_idx;
            }

            let after_open = &self.buffer[self.cursor + OPEN.len()..];
            let Some(close_idx) = after_open.find(CLOSE) else {
                if final_ {
                    events.push(ScanEvent::Text(self.buffer[self.cursor..].to_string()));
                    self.cursor = self.buffer.len();
                }
                break; // wait for more input
            };

            let content = after_open[..close_idx].trim().to_string();
            self.cursor += OPEN.len() + close_idx + CLOSE.len();
            let call = safe_parse(&content);
            events.push(ScanEvent::ToolCall { call, raw: content });
        }

        events
    }
}

pub fn tool_call_from_parsed(parsed: ParsedToolCall) -> ToolCall {
    ToolCall { id: generate_id("call"), name: parsed.name, arguments: parsed.arguments }
}
