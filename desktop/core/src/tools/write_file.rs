use super::{arg_string, err, ok, unified_diff, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::fs;

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create a new file or overwrite an existing file with the given content. Creates parent directories as needed.".to_string(),
            parameters: super::schema(
                json!({
                    "path": {"type": "string", "description": "File path, relative to the working directory or absolute."},
                    "content": {"type": "string", "description": "Full text content to write to the file."}
                }),
                vec!["path".to_string(), "content".to_string()],
            ),
            mutating: true,
        }
    }

    async fn preview(&self, args: &Map<String, Value>, ctx: &ToolContext) -> Option<String> {
        let path = arg_string(args, "path");
        let content = arg_string(args, "content");
        let full_path = ctx.cwd.join(&path);
        let before = fs::read_to_string(&full_path).unwrap_or_default();
        if before == content {
            return Some(format!("No changes to {path}"));
        }
        Some(unified_diff(&path, &before, &content))
    }

    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult {
        let path = arg_string(args, "path");
        if path.is_empty() {
            return err("Missing required argument: path");
        }
        let Some(content) = args.get("content").and_then(|v| v.as_str()) else {
            return err("Missing required argument: content");
        };

        let full_path = ctx.cwd.join(&path);
        if let Some(parent) = full_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return err(format!("Failed to write {path}: {e}"));
            }
        }
        if let Err(e) = fs::write(&full_path, content) {
            return err(format!("Failed to write {path}: {e}"));
        }

        ok(format!("Wrote {} lines to {path}", content.split('\n').count()))
    }
}
