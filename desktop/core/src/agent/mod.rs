mod stream_util;
pub mod system_prompt;
pub mod thinking_filter;
pub mod tool_call_parser;

use crate::tools::registry::{all_tools, tools_by_name};
use crate::tools::ToolContext;
use crate::types::{ChatOptions, Message, Provider, StreamEvent, ToolCall, ToolExecutionResult};
use futures::StreamExt;
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Arc;
use thinking_filter::{strip_thinking_tags, FilterEvent, ThinkingTagFilter};
use tokio::sync::{mpsc, oneshot};
use tool_call_parser::{tool_call_from_parsed, ScanEvent, StreamingToolCallScanner};

const MAX_ITERATIONS: u32 = 25;

#[derive(Debug)]
pub enum AgentEvent {
    TextChunk(String),
    ThinkingChunk(String),
    ToolStart { name: String, args: Map<String, Value> },
    /// The GUI resolves `respond` with the user's Approve/Deny choice.
    NeedsApproval { name: String, preview: String, respond: oneshot::Sender<bool> },
    ToolResult { name: String, output: String, is_error: bool },
    Notice(String),
    /// The turn is over; carries the full updated conversation for the caller to keep.
    Done { messages: Vec<Message> },
}

pub struct RunAgentTurnParams {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub messages: Vec<Message>,
    pub cwd: PathBuf,
    pub auto_approve: bool,
    pub events: mpsc::UnboundedSender<AgentEvent>,
}

fn handle_scan_events(
    events_out: Vec<ScanEvent>,
    sender: &mpsc::UnboundedSender<AgentEvent>,
    fallback_calls: &mut Vec<ToolCall>,
    had_parse_error: &mut bool,
) {
    for scan_event in events_out {
        match scan_event {
            ScanEvent::Text(t) => {
                let _ = sender.send(AgentEvent::TextChunk(t));
            }
            ScanEvent::ToolCall { call: Some(c), .. } => fallback_calls.push(tool_call_from_parsed(c)),
            ScanEvent::ToolCall { call: None, .. } => *had_parse_error = true,
        }
    }
}

fn handle_filter_events(
    events_out: Vec<FilterEvent>,
    sender: &mpsc::UnboundedSender<AgentEvent>,
    scanner: &mut StreamingToolCallScanner,
    fallback_calls: &mut Vec<ToolCall>,
    had_parse_error: &mut bool,
) {
    for filter_event in events_out {
        match filter_event {
            FilterEvent::Thinking(t) => {
                let _ = sender.send(AgentEvent::ThinkingChunk(t));
            }
            FilterEvent::Text(t) => {
                let scan_events = scanner.feed(&t);
                handle_scan_events(scan_events, sender, fallback_calls, had_parse_error);
            }
        }
    }
}

/// Runs the agent until the model produces a plain-text reply with no further tool calls.
/// Intended to be driven from a spawned tokio task; progress is reported entirely through
/// `params.events` rather than a return value.
pub async fn run_agent_turn(params: RunAgentTurnParams) {
    let RunAgentTurnParams { provider, model, mut messages, cwd, auto_approve, events } = params;
    let tools = tools_by_name();
    let tool_defs = all_tools().iter().map(|t| t.definition()).collect::<Vec<_>>();
    let ctx = ToolContext { cwd };

    for _iteration in 0..MAX_ITERATIONS {
        let mut think_filter = ThinkingTagFilter::new();
        let mut scanner = StreamingToolCallScanner::new();
        let mut fallback_calls: Vec<ToolCall> = Vec::new();
        let mut had_parse_error = false;
        let mut full_text = String::new();
        let mut native_tool_calls: Vec<ToolCall> = Vec::new();

        let chat_result = provider
            .chat(ChatOptions { model: model.clone(), messages: messages.clone(), tools: tool_defs.clone(), use_native_tools: true })
            .await;

        let mut stream = match chat_result {
            Ok(s) => s,
            Err(e) => {
                let _ = events.send(AgentEvent::Notice(format!("Error: {e}")));
                let _ = events.send(AgentEvent::Done { messages });
                return;
            }
        };

        let mut stream_error: Option<String> = None;

        while let Some(item) = stream.next().await {
            match item {
                Ok(StreamEvent::Text(text)) => {
                    let filter_events = think_filter.feed(&text);
                    handle_filter_events(filter_events, &events, &mut scanner, &mut fallback_calls, &mut had_parse_error);
                }
                Ok(StreamEvent::ToolCalls(tc)) => native_tool_calls = tc,
                Ok(StreamEvent::Done { text, tool_calls }) => {
                    full_text = text;
                    if !tool_calls.is_empty() {
                        native_tool_calls = tool_calls;
                    }
                }
                Err(e) => {
                    stream_error = Some(e.to_string());
                    break;
                }
            }
        }

        if let Some(e) = stream_error {
            let _ = events.send(AgentEvent::Notice(format!("Error: {e}")));
            let _ = events.send(AgentEvent::Done { messages });
            return;
        }

        let filter_events = think_filter.finish();
        handle_filter_events(filter_events, &events, &mut scanner, &mut fallback_calls, &mut had_parse_error);
        let scan_events = scanner.finish();
        handle_scan_events(scan_events, &events, &mut fallback_calls, &mut had_parse_error);

        let tool_calls = if !native_tool_calls.is_empty() { native_tool_calls } else { fallback_calls };

        messages.push(Message::assistant(
            strip_thinking_tags(&full_text),
            if tool_calls.is_empty() { None } else { Some(tool_calls.clone()) },
        ));

        if tool_calls.is_empty() {
            if had_parse_error {
                messages.push(Message::user(
                    "Your tool_call block could not be parsed as JSON. Please retry with a single valid JSON object: {\"name\": \"...\", \"arguments\": {...}}",
                ));
                continue;
            }
            let _ = events.send(AgentEvent::Done { messages });
            return;
        }

        for call in tool_calls {
            let Some(tool) = tools.get(&call.name) else {
                let output = format!(
                    "Unknown tool \"{}\". Available tools: {}",
                    call.name,
                    tools.keys().cloned().collect::<Vec<_>>().join(", ")
                );
                let _ = events.send(AgentEvent::ToolResult { name: call.name.clone(), output: output.clone(), is_error: true });
                messages.push(Message::tool_result(output, call.id, call.name));
                continue;
            };

            let _ = events.send(AgentEvent::ToolStart { name: call.name.clone(), args: call.arguments.clone() });

            let definition = tool.definition();
            let result: ToolExecutionResult = if definition.mutating && !auto_approve {
                let preview = tool
                    .preview(&call.arguments, &ctx)
                    .await
                    .unwrap_or_else(|| serde_json::to_string(&call.arguments).unwrap_or_default());
                let (tx, rx) = oneshot::channel();
                let _ = events.send(AgentEvent::NeedsApproval { name: call.name.clone(), preview, respond: tx });
                let approved = rx.await.unwrap_or(false);
                if approved {
                    tool.execute(&call.arguments, &ctx).await
                } else {
                    ToolExecutionResult { output: "User declined to run this tool.".to_string(), is_error: true }
                }
            } else {
                tool.execute(&call.arguments, &ctx).await
            };

            let _ = events.send(AgentEvent::ToolResult {
                name: call.name.clone(),
                output: result.output.clone(),
                is_error: result.is_error,
            });
            messages.push(Message::tool_result(result.output, call.id, call.name));
        }
    }

    let _ = events.send(AgentEvent::Notice(format!(
        "Stopped after {MAX_ITERATIONS} tool-call iterations to avoid an infinite loop."
    )));
    let _ = events.send(AgentEvent::Done { messages });
}
