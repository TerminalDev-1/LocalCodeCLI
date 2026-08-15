use super::{arg_string, err, ok, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::{json, Map, Value};

pub struct GlobTool;

fn is_excluded(path: &std::path::Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str();
        s == "node_modules" || s == ".git"
    })
}

#[async_trait]
impl Tool for GlobTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern, e.g. 'src/**/*.ts'. Returns matching paths.".to_string(),
            parameters: super::schema(
                json!({
                    "pattern": {"type": "string", "description": "Glob pattern to match files against."}
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

        let walker = match globwalk::GlobWalkerBuilder::from_patterns(&ctx.cwd, &[pattern])
            .file_type(globwalk::FileType::FILE)
            .build()
        {
            Ok(w) => w,
            Err(e) => return err(format!("Glob failed: {e}")),
        };

        let mut matches: Vec<String> = walker
            .filter_map(|entry| entry.ok())
            .filter(|entry| !is_excluded(entry.path()))
            .filter_map(|entry| {
                entry.path().strip_prefix(&ctx.cwd).ok().map(|p| p.to_string_lossy().replace('\\', "/"))
            })
            .collect();
        matches.sort();

        ok(if matches.is_empty() { "(no matches)".to_string() } else { matches.join("\n") })
    }
}
