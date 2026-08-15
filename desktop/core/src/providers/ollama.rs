use super::stream_lines::stream_lines;
use crate::types::{ChatOptions, ChatStream, Message, Provider, ProviderConfig, ProviderError, StreamEvent, ToolCall, ToolDefinition};
use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_tool_call_id() -> String {
    let count = CALL_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    format!("call_{millis}_{count}")
}

#[derive(Debug, Deserialize)]
struct OllamaChatChunk {
    message: Option<OllamaMessage>,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    #[serde(default)]
    arguments: Option<Map<String, Value>>,
}

fn to_ollama_messages(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| {
            let role = serde_json::to_value(m.role).unwrap();
            json!({ "role": role, "content": m.content })
        })
        .collect()
}

fn to_ollama_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| json!({ "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters } }))
        .collect()
}

pub struct OllamaProvider {
    id: String,
    label: String,
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: &ProviderConfig) -> Self {
        Self {
            id: config.id.clone(),
            label: config.label.clone().unwrap_or_else(|| "Ollama".to_string()),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn label(&self) -> &str {
        &self.label
    }

    async fn chat(&self, options: ChatOptions) -> Result<ChatStream, ProviderError> {
        let mut body = json!({
            "model": options.model,
            "messages": to_ollama_messages(&options.messages),
            "stream": true,
        });
        if options.use_native_tools && !options.tools.is_empty() {
            body["tools"] = Value::Array(to_ollama_tools(&options.tools));
        }

        let res = self.client.post(format!("{}/api/chat", self.base_url)).json(&body).send().await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!("Ollama request failed ({status}): {text}")));
        }

        let mut lines = stream_lines(res);

        let stream = async_stream::stream! {
            let mut full_text = String::new();
            let mut all_tool_calls: Vec<ToolCall> = Vec::new();

            while let Some(line_result) = lines.next().await {
                let line = match line_result {
                    Ok(l) => l,
                    Err(_) => break,
                };

                let chunk: OllamaChatChunk = match serde_json::from_str(&line) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if let Some(content) = chunk.message.as_ref().map(|m| m.content.clone()).filter(|c| !c.is_empty()) {
                    full_text.push_str(&content);
                    yield Ok(StreamEvent::Text(content));
                }

                if let Some(tcs) = chunk.message.as_ref().and_then(|m| m.tool_calls.clone()) {
                    if !tcs.is_empty() {
                        let parsed: Vec<ToolCall> = tcs
                            .into_iter()
                            .map(|tc| ToolCall {
                                id: next_tool_call_id(),
                                name: tc.function.name,
                                arguments: tc.function.arguments.unwrap_or_default(),
                            })
                            .collect();
                        all_tool_calls.extend(parsed.clone());
                        yield Ok(StreamEvent::ToolCalls(parsed));
                    }
                }

                if chunk.done {
                    break;
                }
            }

            yield Ok(StreamEvent::Done { text: full_text, tool_calls: all_tool_calls });
        };

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let res = self.client.get(format!("{}/api/tags", self.base_url)).send().await?;
        if !res.status().is_success() {
            return Ok(vec![]);
        }

        #[derive(Deserialize)]
        struct Tags {
            #[serde(default)]
            models: Vec<ModelEntry>,
        }
        #[derive(Deserialize)]
        struct ModelEntry {
            name: String,
        }

        let data: Tags = res.json().await?;
        Ok(data.models.into_iter().map(|m| m.name).collect())
    }
}
