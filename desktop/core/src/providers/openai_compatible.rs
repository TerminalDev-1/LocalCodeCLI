use super::stream_lines::stream_lines;
use crate::types::{
    ChatOptions, ChatStream, Message, Provider, ProviderConfig, ProviderError, Role, StreamEvent, ToolCall, ToolDefinition,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
struct OpenAiChunk {
    #[serde(default)]
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiChoice {
    #[serde(default)]
    delta: OpenAiDelta,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiDeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDeltaToolCall {
    index: i64,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAiDeltaFunction>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiDeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

fn to_openai_messages(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| match m.role {
            Role::Tool => json!({
                "role": "tool",
                "tool_call_id": m.tool_call_id.clone().unwrap_or_else(|| "call_unknown".to_string()),
                "content": m.content,
            }),
            Role::Assistant if m.tool_calls.as_ref().map(|tc| !tc.is_empty()).unwrap_or(false) => {
                let tool_calls: Vec<Value> = m
                    .tool_calls
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": { "name": tc.name, "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default() },
                        })
                    })
                    .collect();
                json!({ "role": "assistant", "content": m.content, "tool_calls": tool_calls })
            }
            _ => json!({ "role": serde_json::to_value(m.role).unwrap(), "content": m.content }),
        })
        .collect()
}

fn to_openai_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| json!({ "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters } }))
        .collect()
}

pub struct OpenAiCompatibleProvider {
    id: String,
    label: String,
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: &ProviderConfig) -> Self {
        Self {
            id: config.id.clone(),
            label: config.label.clone().unwrap_or_else(|| "OpenAI-compatible".to_string()),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Provider for OpenAiCompatibleProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn label(&self) -> &str {
        &self.label
    }

    async fn chat(&self, options: ChatOptions) -> Result<ChatStream, ProviderError> {
        let mut body = json!({
            "model": options.model,
            "messages": to_openai_messages(&options.messages),
            "stream": true,
        });
        if options.use_native_tools && !options.tools.is_empty() {
            body["tools"] = Value::Array(to_openai_tools(&options.tools));
        }

        let mut req = self.client.post(format!("{}/chat/completions", self.base_url)).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let res = req.send().await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            let label = self.label.clone();
            return Err(ProviderError::Message(format!("{label} request failed ({status}): {text}")));
        }

        let mut lines = stream_lines(res);

        let stream = async_stream::stream! {
            let mut full_text = String::new();
            let mut buffers: HashMap<i64, (String, String, String)> = HashMap::new();
            let mut order: Vec<i64> = Vec::new();

            while let Some(line_result) = lines.next().await {
                let line = match line_result {
                    Ok(l) => l,
                    Err(_) => break,
                };
                let Some(payload) = line.strip_prefix("data:") else { continue };
                let payload = payload.trim();
                if payload == "[DONE]" {
                    break;
                }

                let chunk: OpenAiChunk = match serde_json::from_str(payload) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let Some(choice) = chunk.choices.into_iter().next() else { continue };
                let delta = choice.delta;

                if let Some(content) = delta.content.filter(|c| !c.is_empty()) {
                    full_text.push_str(&content);
                    yield Ok(StreamEvent::Text(content));
                }

                if let Some(tcs) = delta.tool_calls {
                    for tc in tcs {
                        if let std::collections::hash_map::Entry::Vacant(e) = buffers.entry(tc.index) {
                            order.push(tc.index);
                            e.insert((tc.id.clone().unwrap_or_else(|| format!("call_{}", tc.index)), String::new(), String::new()));
                        }
                        let entry = buffers.get_mut(&tc.index).unwrap();
                        if let Some(id) = tc.id {
                            entry.0 = id;
                        }
                        if let Some(f) = &tc.function {
                            if let Some(name) = &f.name {
                                entry.1.push_str(name);
                            }
                            if let Some(args) = &f.arguments {
                                entry.2.push_str(args);
                            }
                        }
                    }
                }
            }

            let tool_calls: Vec<ToolCall> = order
                .iter()
                .filter_map(|idx| buffers.get(idx))
                .map(|(id, name, args)| {
                    let arguments: Map<String, Value> = if args.is_empty() {
                        Map::new()
                    } else {
                        serde_json::from_str::<Value>(args).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default()
                    };
                    ToolCall { id: id.clone(), name: name.clone(), arguments }
                })
                .collect();

            if !tool_calls.is_empty() {
                yield Ok(StreamEvent::ToolCalls(tool_calls.clone()));
            }

            yield Ok(StreamEvent::Done { text: full_text, tool_calls });
        };

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let mut req = self.client.get(format!("{}/models", self.base_url));
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        #[derive(Deserialize)]
        struct ModelsResponse {
            #[serde(default)]
            data: Vec<ModelEntry>,
        }
        #[derive(Deserialize)]
        struct ModelEntry {
            id: String,
        }

        let result: Result<Vec<String>, ProviderError> = async {
            let res = req.send().await?;
            if !res.status().is_success() {
                return Ok(vec![]);
            }
            let data: ModelsResponse = res.json().await?;
            Ok(data.data.into_iter().map(|m| m.id).collect())
        }
        .await;

        Ok(result.unwrap_or_default())
    }
}
