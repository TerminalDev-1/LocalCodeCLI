use super::{arg_string, err, ok, Tool, ToolContext};
use crate::types::{ToolDefinition, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const MAX_OUTPUT_CHARS: usize = 20_000;
const TIMEOUT_SECS: u64 = 120;

fn truncate(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_OUTPUT_CHARS {
        return text.to_string();
    }
    let truncated: String = text.chars().take(MAX_OUTPUT_CHARS).collect();
    format!("{truncated}\n... (truncated, {} more characters)", count - MAX_OUTPUT_CHARS)
}

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".to_string(),
            description: "Run a shell command in the project working directory and return its stdout/stderr. Uses the OS default shell (PowerShell/cmd on Windows, sh elsewhere).".to_string(),
            parameters: super::schema(
                json!({
                    "command": {"type": "string", "description": "The shell command to run."}
                }),
                vec!["command".to_string()],
            ),
            mutating: true,
        }
    }

    async fn preview(&self, args: &Map<String, Value>, _ctx: &ToolContext) -> Option<String> {
        Some(format!("$ {}", arg_string(args, "command")))
    }

    async fn execute(&self, args: &Map<String, Value>, ctx: &ToolContext) -> ToolExecutionResult {
        let command = arg_string(args, "command");
        if command.is_empty() {
            return err("Missing required argument: command");
        }

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };
        cmd.current_dir(&ctx.cwd).stdout(Stdio::piped()).stderr(Stdio::piped());

        match tokio::time::timeout(Duration::from_secs(TIMEOUT_SECS), cmd.output()).await {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined =
                    [stdout.trim(), stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
                let text = if combined.is_empty() { "(no output)".to_string() } else { truncate(&combined) };
                if out.status.success() {
                    ok(text)
                } else {
                    err(format!(
                        "{text}\n\nCommand exited with error: exit status {}",
                        out.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string())
                    ))
                }
            }
            Ok(Err(e)) => err(format!("Failed to run command: {e}")),
            Err(_) => err(format!("Command timed out after {TIMEOUT_SECS}s")),
        }
    }
}
