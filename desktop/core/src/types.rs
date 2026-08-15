//! Shared types used across providers, tools, and the agent loop.

use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// Present on assistant messages that invoked tools natively.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Present on tool-result messages; links back to the ToolCall.id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Present on tool-result messages for providers that key by name instead of id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into(), tool_calls: None, tool_call_id: None, tool_name: None }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into(), tool_calls: None, tool_call_id: None, tool_name: None }
    }

    pub fn assistant(content: impl Into<String>, tool_calls: Option<Vec<ToolCall>>) -> Self {
        Self { role: Role::Assistant, content: content.into(), tool_calls, tool_call_id: None, tool_name: None }
    }

    pub fn tool_result(content: impl Into<String>, tool_call_id: String, tool_name: String) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
            tool_name: Some(tool_name),
        }
    }
}

/// JSON-schema-shaped tool parameter description, sent to providers verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameterSchema {
    #[serde(rename = "type")]
    pub kind: String,
    pub properties: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameterSchema,
    /// Whether this tool mutates the filesystem or runs commands, and therefore needs approval.
    pub mutating: bool,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Text(String),
    ToolCalls(Vec<ToolCall>),
    /// Full assistant text accumulated over the stream, plus any tool calls.
    Done { text: String, tool_calls: Vec<ToolCall> },
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

pub struct ChatOptions {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    /// Whether to also send tools in provider-native format (best-effort). Prompt-based tool
    /// calls always work regardless.
    pub use_native_tools: bool,
}

pub type ChatStream = BoxStream<'static, Result<StreamEvent, ProviderError>>;

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    /// Human-readable label, e.g. "Ollama" or "LM Studio".
    fn label(&self) -> &str;
    /// Stream a chat completion. Yields text chunks and a final done event.
    async fn chat(&self, options: ChatOptions) -> Result<ChatStream, ProviderError>;
    /// List model names currently available from this provider, if it can be queried.
    async fn list_models(&self) -> Result<Vec<String>, ProviderError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ProviderType,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ProviderConfig {
    pub fn display_label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCodeConfig {
    pub providers: Vec<ProviderConfig>,
    pub default_provider: String,
    pub default_model: String,
    #[serde(default)]
    pub auto_approve: bool,
}
