//! Adapts `local_code_core::agent::AgentEvent` (which carries a non-`Clone` oneshot sender)
//! into an iced-`Message`-friendly shape, and turns a spawned agent turn into an `iced::Task`
//! that emits one `Message` per event as they arrive.

use futures::StreamExt;
use local_code_core::agent::{run_agent_turn, AgentEvent, RunAgentTurnParams};
use local_code_core::types::{Message as CoreMessage, Provider};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

/// Clone-able wrapper around a one-shot approval channel so it can live inside an iced
/// `Message`. Only the first `respond()` call has any effect.
#[derive(Clone)]
pub struct ApprovalResponder(Arc<Mutex<Option<oneshot::Sender<bool>>>>);

impl std::fmt::Debug for ApprovalResponder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApprovalResponder(..)")
    }
}

impl ApprovalResponder {
    fn new(sender: oneshot::Sender<bool>) -> Self {
        Self(Arc::new(Mutex::new(Some(sender))))
    }

    pub fn respond(&self, approved: bool) {
        if let Some(tx) = self.0.lock().unwrap().take() {
            let _ = tx.send(approved);
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppAgentEvent {
    TextChunk(String),
    ThinkingChunk(String),
    ToolStart { name: String, args: Map<String, Value> },
    /// Correlated to the most recently started tool by transcript position, not by name.
    NeedsApproval { preview: String, respond: ApprovalResponder },
    ToolResult { output: String, is_error: bool },
    Notice(String),
    Done { messages: Vec<CoreMessage> },
}

fn adapt(event: AgentEvent) -> AppAgentEvent {
    match event {
        AgentEvent::TextChunk(t) => AppAgentEvent::TextChunk(t),
        AgentEvent::ThinkingChunk(t) => AppAgentEvent::ThinkingChunk(t),
        AgentEvent::ToolStart { name, args } => AppAgentEvent::ToolStart { name, args },
        AgentEvent::NeedsApproval { preview, respond, .. } => {
            AppAgentEvent::NeedsApproval { preview, respond: ApprovalResponder::new(respond) }
        }
        AgentEvent::ToolResult { output, is_error, .. } => AppAgentEvent::ToolResult { output, is_error },
        AgentEvent::Notice(n) => AppAgentEvent::Notice(n),
        AgentEvent::Done { messages } => AppAgentEvent::Done { messages },
    }
}

pub struct TurnRequest {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub messages: Vec<CoreMessage>,
    pub cwd: PathBuf,
    pub auto_approve: bool,
}

/// Spawns the agent turn on its own tokio task and returns a stream of adapted events, ending
/// once the turn sends its `Done` event and the channel closes.
pub fn agent_event_stream(req: TurnRequest) -> impl futures::Stream<Item = AppAgentEvent> {
    let (tx, rx) = mpsc::unbounded_channel::<AgentEvent>();
    let params = RunAgentTurnParams {
        provider: req.provider,
        model: req.model,
        messages: req.messages,
        cwd: req.cwd,
        auto_approve: req.auto_approve,
        events: tx,
    };
    tokio::spawn(run_agent_turn(params));

    futures::stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|event| (adapt(event), rx)) })
        .boxed()
}
