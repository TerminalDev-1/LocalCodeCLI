use super::{arg_number, arg_string, err, ok, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::fs;

const MAX_LINES: usize = 2000;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description:
                "Read a text file from disk. Returns content with line numbers. Use offset/limit for large files."
                    .to_string(),
            parameters: super::schema(
                json!({
                    "path": {"type": "string", "description": "File path, relative to the working directory or absolute."},
                    "offset": {"type": "number", "description": "1-indexed line number to start reading from (optional)."},
                    "limit": {"type": "number", "description": "Maximum number of lines to read (optional, default 2000)."}
                }),
                vec!["path".to_string()],
            ),
            mutating: false,
        }
    }

    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult {
        let path = arg_string(args, "path");
        if path.is_empty() {
            return err("Missing required argument: path");
        }

        let full_path = ctx.cwd.join(&path);
        if !full_path.exists() {
            return err(format!("File not found: {path}"));
        }
        if full_path.is_dir() {
            return err(format!("{path} is a directory, not a file."));
        }

        let raw = match fs::read_to_string(&full_path) {
            Ok(s) => s,
            Err(e) => return err(format!("Failed to read {path}: {e}")),
        };

        let lines: Vec<&str> = raw.split('\n').collect();
        let offset = arg_number(args, "offset").filter(|v| *v > 0.0).map(|v| v as usize).unwrap_or(1);
        let limit = arg_number(args, "limit").filter(|v| *v > 0.0).map(|v| v as usize).unwrap_or(MAX_LINES);

        let start = offset.saturating_sub(1).min(lines.len());
        let end = start.saturating_add(limit).min(lines.len());
        let slice = &lines[start..end];

        let numbered = slice
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}\t{}", offset + i, line))
            .collect::<Vec<_>>()
            .join("\n");

        let truncated = end < lines.len();
        ok(if truncated {
            format!("{numbered}\n... ({} more lines)", lines.len() - end)
        } else {
            numbered
        })
    }
}
