//! Projects + chats: the desktop app's own persisted workspace, separate from the
//! CLI-compatible `~/.local-code/config.json` (which stays just provider/model config).

use local_code_core::id::generate_id;
use local_code_core::types::Message as CoreMessage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRecord {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub provider_id: String,
    pub model: String,
    pub messages: Vec<CoreMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ChatRecord {
    pub fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| "New chat".to_string())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub projects: Vec<ProjectRecord>,
    #[serde(default)]
    pub chats: Vec<ChatRecord>,
    #[serde(default)]
    pub active_project: Option<String>,
    #[serde(default)]
    pub active_chat: Option<String>,
}

impl Workspace {
    pub fn add_project(&mut self, path: PathBuf) -> String {
        if let Some(existing) = self.projects.iter().find(|p| p.path == path) {
            return existing.id.clone();
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string_lossy().into_owned());
        let id = generate_id("project");
        self.projects.push(ProjectRecord { id: id.clone(), name, path });
        self.active_project = Some(id.clone());
        id
    }

    pub fn add_chat(&mut self, project_id: &str, provider_id: String, model: String) -> String {
        let id = generate_id("chat");
        let now = now_millis();
        self.chats.push(ChatRecord {
            id: id.clone(),
            project_id: project_id.to_string(),
            title: None,
            provider_id,
            model,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        });
        self.active_chat = Some(id.clone());
        id
    }

    pub fn project(&self, id: &str) -> Option<&ProjectRecord> {
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn chat(&self, id: &str) -> Option<&ChatRecord> {
        self.chats.iter().find(|c| c.id == id)
    }

    pub fn chat_mut(&mut self, id: &str) -> Option<&mut ChatRecord> {
        self.chats.iter_mut().find(|c| c.id == id)
    }

    pub fn chats_for_project(&self, project_id: &str) -> Vec<&ChatRecord> {
        let mut chats: Vec<&ChatRecord> = self.chats.iter().filter(|c| c.project_id == project_id).collect();
        chats.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        chats
    }

    pub fn most_recent_chat_for_project(&self, project_id: &str) -> Option<&ChatRecord> {
        self.chats_for_project(project_id).into_iter().next()
    }
}

fn workspace_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".local-code").join("desktop")
}

fn workspace_path() -> PathBuf {
    workspace_dir().join("workspace.json")
}

fn read_json(path: &Path) -> Option<Workspace> {
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn load_workspace() -> Workspace {
    read_json(&workspace_path()).unwrap_or_default()
}

pub fn save_workspace(workspace: &Workspace) -> std::io::Result<()> {
    let dir = workspace_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let json = serde_json::to_string_pretty(workspace).unwrap_or_default();
    fs::write(workspace_path(), json)
}
