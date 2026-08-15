use super::{arg_string, err, ok, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::fs;

pub struct GrepTool;

const MAX_FILE_BYTES: u64 = 1_000_000;
const MAX_MATCHES: usize = 200;

fn is_excluded(path: &std::path::Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str();
        s == "node_modules" || s == ".git" || s == "dist"
    })
}

#[async_trait]
impl Tool for GrepTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search file contents for a regular expression. Returns matching file:line:text entries."
                .to_string(),
            parameters: super::schema(
                json!({
                    "pattern": {"type": "string", "description": "Regular expression to search for."},
                    "path": {"type": "string", "description": "Directory or glob to restrict the search to (optional, defaults to the whole project)."}
                }),
                vec!["pattern".to_string()],
            ),
            mutating: false,
        }
    }

    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult {
        let pattern = arg_string(args, "pattern");
        if pattern.is_empty() {
            return err("Missing required argument: pattern");
        }

        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => return err(format!("Invalid regular expression: {e}")),
        };

        let path_arg = arg_string(args, "path");
        let glob_pattern =
            if path_arg.is_empty() { "**/*".to_string() } else { format!("{}/**/*", path_arg.trim_end_matches('/')) };

        let walker = match globwalk::GlobWalkerBuilder::from_patterns(&ctx.cwd, &[glob_pattern])
            .file_type(globwalk::FileType::FILE)
            .build()
        {
            Ok(w) => w,
            Err(e) => return err(format!("Search failed: {e}")),
        };

        let mut results: Vec<String> = Vec::new();
        for entry in walker.filter_map(|e| e.ok()) {
            if results.len() >= MAX_MATCHES {
                break;
            }
            let full_path = entry.path();
            if is_excluded(full_path) {
                continue;
            }

            let Ok(metadata) = fs::metadata(full_path) else { continue };
            if metadata.len() > MAX_FILE_BYTES {
                continue;
            }
            let Ok(content) = fs::read_to_string(full_path) else { continue };
            let rel = full_path.strip_prefix(&ctx.cwd).unwrap_or(full_path).to_string_lossy().replace('\\', "/");

            for (i, line) in content.split('\n').enumerate() {
                if regex.is_match(line) {
                    results.push(format!("{rel}:{}:{line}", i + 1));
                    if results.len() >= MAX_MATCHES {
                        break;
                    }
                }
            }
        }

        if results.is_empty() {
            return ok("(no matches)");
        }
        let truncated = results.len() >= MAX_MATCHES;
        ok(if truncated { format!("{}\n... (truncated at {MAX_MATCHES} matches)", results.join("\n")) } else { results.join("\n") })
    }
}
