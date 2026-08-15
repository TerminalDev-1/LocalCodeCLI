//! Detect/start/pull helpers for a local Ollama install.
//!
//! On Windows, a plain `ollama` lookup can miss even when the app works fine from an
//! interactive shell — PATH picked up by an installer often isn't visible to a process
//! that was already running. Fall back to the installer's default locations before
//! giving up.

use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc::UnboundedSender, OnceCell};

pub const DEFAULT_WAIT_TIMEOUT_MS: u64 = 8000;

fn ollama_candidates() -> Vec<String> {
    let mut candidates = vec!["ollama".to_string()];
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(format!("{local_app_data}\\Programs\\Ollama\\ollama.exe"));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(format!("{program_files}\\Ollama\\ollama.exe"));
        }
    } else {
        candidates.push("/usr/local/bin/ollama".to_string());
        candidates.push("/opt/homebrew/bin/ollama".to_string());
    }
    candidates
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_window(_cmd: &mut Command) {}

async fn check_version(candidate: &str) -> bool {
    let mut cmd = Command::new(candidate);
    cmd.arg("--version").stdout(Stdio::null()).stderr(Stdio::null());
    no_window(&mut cmd);
    matches!(cmd.status().await, Ok(status) if status.success())
}

// Only caches a *successful* resolution — same as the TS version, so if Ollama gets
// installed mid-session a later call will still find it instead of being stuck on `None`.
static RESOLVED_OLLAMA_PATH: OnceCell<String> = OnceCell::const_new();

async fn resolve_ollama_path() -> Option<String> {
    RESOLVED_OLLAMA_PATH
        .get_or_try_init(|| async {
            for candidate in ollama_candidates() {
                if check_version(&candidate).await {
                    return Ok(candidate);
                }
            }
            Err(())
        })
        .await
        .ok()
        .cloned()
}

pub async fn is_ollama_installed() -> bool {
    resolve_ollama_path().await.is_some()
}

pub async fn is_ollama_reachable(base_url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder().timeout(Duration::from_millis(1500)).build() else {
        return false;
    };
    matches!(
        client.get(format!("{base_url}/api/tags")).send().await,
        Ok(res) if res.status().is_success()
    )
}

/// Starts `ollama serve` detached from this process; caller should poll with `wait_for_ollama`.
pub async fn start_ollama_server() {
    let ollama_path = resolve_ollama_path().await.unwrap_or_else(|| "ollama".to_string());
    let mut cmd = Command::new(&ollama_path);
    cmd.arg("serve").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    no_window(&mut cmd);
    let _ = cmd.spawn();
}

pub async fn wait_for_ollama(base_url: &str, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        if is_ollama_reachable(base_url).await {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
}

/// Runs `ollama pull <model>`, forwarding each output line to `progress` so a GUI can show
/// Ollama's pull progress live (the terminal CLI just inherited stdio for this; there's no
/// terminal to inherit into here).
pub async fn pull_model(model_name: &str, progress: UnboundedSender<String>) -> bool {
    let ollama_path = resolve_ollama_path().await.unwrap_or_else(|| "ollama".to_string());
    let mut cmd = Command::new(&ollama_path);
    cmd.arg("pull").arg(model_name).stdout(Stdio::piped()).stderr(Stdio::piped());
    no_window(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(stdout) = child.stdout.take() {
        let tx = progress.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = progress.send(line);
            }
        });
    }

    matches!(child.wait().await, Ok(status) if status.success())
}
