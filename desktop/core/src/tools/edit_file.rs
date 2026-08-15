use super::{arg_string, err, ok, unified_diff, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::fs;

pub struct EditFileTool;

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

#[async_trait]
impl Tool for EditFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Replace an exact snippet of text in a file with new text. old_string must match exactly once in the file. Use this for targeted edits instead of rewriting the whole file.".to_string(),
            parameters: super::schema(
                json!({
                    "path": {"type": "string", "description": "File path, relative to the working directory or absolute."},
                    "old_string": {"type": "string", "description": "Exact text to find. Must be unique in the file."},
                    "new_string": {"type": "string", "description": "Text to replace it with."}
                }),
                vec!["path".to_string(), "old_string".to_string(), "new_string".to_string()],
            ),
            mutating: true,
        }
    }

    async fn preview(&self, args: &Map<String, Value>, ctx: &ToolContext) -> Option<String> {
        let path = arg_string(args, "path");
        let old_string = arg_string(args, "old_string");
        let new_string = arg_string(args, "new_string");
        let full_path = ctx.cwd.join(&path);
        if !full_path.exists() {
            return Some(format!("File not found: {path}"));
        }
        let before = fs::read_to_string(&full_path).unwrap_or_default();
        let after = before.replacen(&old_string, &new_string, 1);
        Some(unified_diff(&path, &before, &after))
    }

    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult {
        let path = arg_string(args, "path");
        if path.is_empty() {
            return err("Missing required argument: path");
        }
        let Some(old_string) = args.get("old_string").and_then(|v| v.as_str()) else {
            return err("Missing required argument: old_string");
        };
        let Some(new_string) = args.get("new_string").and_then(|v| v.as_str()) else {
            return err("Missing required argument: new_string");
        };

        let full_path = ctx.cwd.join(&path);
        if !full_path.exists() {
            return err(format!("File not found: {path}"));
        }

        let before = match fs::read_to_string(&full_path) {
            Ok(s) => s,
            Err(e) => return err(format!("Failed to read {path}: {e}")),
        };

        let occurrences = count_occurrences(&before, old_string);
        if occurrences == 0 {
            return err(format!(
                "old_string not found in {path}. Make sure it matches exactly, including whitespace."
            ));
        }
        if occurrences > 1 {
            return err(format!(
                "old_string matches {occurrences} times in {path}. Include more surrounding context so it matches exactly once."
            ));
        }

        let after = before.replacen(old_string, new_string, 1);
        if let Err(e) = fs::write(&full_path, &after) {
            return err(format!("Failed to write {path}: {e}"));
        }

        ok(format!("Edited {path}"))
    }
}
