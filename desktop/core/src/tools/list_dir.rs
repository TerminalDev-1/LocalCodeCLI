use super::{arg_string, err, ok, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::fs;

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".to_string(),
            description: "List files and subdirectories at a given path (non-recursive).".to_string(),
            parameters: super::schema(
                json!({
                    "path": {"type": "string", "description": "Directory path, relative to the working directory or absolute. Defaults to '.'."}
                }),
                vec![],
            ),
            mutating: false,
        }
    }

    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult {
        let path_arg = arg_string(args, "path");
        let path = if path_arg.is_empty() { ".".to_string() } else { path_arg };
        let full_path = ctx.cwd.join(&path);
        if !full_path.exists() {
            return err(format!("Path not found: {path}"));
        }
        if !full_path.is_dir() {
            return err(format!("{path} is not a directory."));
        }

        let read_dir = match fs::read_dir(&full_path) {
            Ok(rd) => rd,
            Err(e) => return err(format!("Failed to list {path}: {e}")),
        };

        let mut entries: Vec<(String, bool)> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                name != "node_modules" && name != ".git"
            })
            .map(|e| {
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                (e.file_name().to_string_lossy().into_owned(), is_dir)
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let lines: Vec<String> =
            entries.into_iter().map(|(name, is_dir)| if is_dir { format!("{name}/") } else { name }).collect();
        ok(if lines.is_empty() { "(empty directory)".to_string() } else { lines.join("\n") })
    }
}
