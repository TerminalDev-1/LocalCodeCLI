use crate::types::{ToolDefinition, ToolExecutionResult, ToolParameterSchema};
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::path::PathBuf;

mod bash;
mod edit_file;
mod glob;
mod grep;
mod list_dir;
mod read_file;
pub mod registry;
mod write_file;

pub use bash::BashTool;
pub use edit_file::EditFileTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use list_dir::ListDirTool;
pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;

pub struct ToolContext {
    pub cwd: PathBuf,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult;
    /// Optional human-readable preview (e.g. a diff) shown before asking for approval.
    async fn preview(&self, _args: &Map<String, Value>, _ctx: &ToolContext) -> Option<String> {
        None
    }
}

pub fn ok(output: impl Into<String>) -> ToolExecutionResult {
    ToolExecutionResult { output: output.into(), is_error: false }
}

pub fn err(output: impl Into<String>) -> ToolExecutionResult {
    ToolExecutionResult { output: output.into(), is_error: true }
}

pub fn arg_string(args: &Map<String, Value>, key: &str) -> String {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn arg_number(args: &Map<String, Value>, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

pub fn schema(properties: Value, required: Vec<String>) -> ToolParameterSchema {
    ToolParameterSchema {
        kind: "object".to_string(),
        properties: properties.as_object().cloned().unwrap_or_default(),
        required: if required.is_empty() { None } else { Some(required) },
    }
}

pub fn unified_diff(path: &str, before: &str, after: &str) -> String {
    similar::TextDiff::from_lines(before, after).unified_diff().header(path, path).to_string()
}
