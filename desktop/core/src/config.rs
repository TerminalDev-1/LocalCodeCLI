//! Load/save `~/.local-code/config.json` + project overrides (`.local-code.json` in cwd).
//!
//! Merge semantics mirror the original TS implementation: providers are merged by id
//! across (defaults, user config, project config), later lists override fields present
//! in earlier ones but a provider keeps the list position of its first appearance —
//! same as repeatedly `Map.set()`-ing an already-present key in JS.

use crate::types::{LocalCodeConfig, ProviderConfig, ProviderType};
use serde::{de::DeserializeOwned, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};

fn config_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".local-code")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn project_config_path(cwd: &Path) -> PathBuf {
    cwd.join(".local-code.json")
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawProviderConfig {
    id: Option<String>,
    #[serde(rename = "type")]
    kind: Option<ProviderType>,
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(rename = "apiKey")]
    api_key: Option<String>,
    label: Option<String>,
}

impl RawProviderConfig {
    fn merge_from(&mut self, other: &RawProviderConfig) {
        if other.kind.is_some() {
            self.kind = other.kind;
        }
        if other.base_url.is_some() {
            self.base_url = other.base_url.clone();
        }
        if other.api_key.is_some() {
            self.api_key = other.api_key.clone();
        }
        if other.label.is_some() {
            self.label = other.label.clone();
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawConfig {
    providers: Option<Vec<RawProviderConfig>>,
    #[serde(rename = "defaultProvider")]
    default_provider: Option<String>,
    #[serde(rename = "defaultModel")]
    default_model: Option<String>,
    #[serde(rename = "autoApprove")]
    auto_approve: Option<bool>,
}

fn default_providers() -> Vec<RawProviderConfig> {
    vec![
        RawProviderConfig {
            id: Some("ollama".to_string()),
            kind: Some(ProviderType::Ollama),
            base_url: Some("http://localhost:11434".to_string()),
            api_key: None,
            label: Some("Ollama".to_string()),
        },
        RawProviderConfig {
            id: Some("lmstudio".to_string()),
            kind: Some(ProviderType::OpenAiCompatible),
            base_url: Some("http://localhost:1234/v1".to_string()),
            api_key: None,
            label: Some("LM Studio".to_string()),
        },
    ]
}

fn merge_providers(lists: Vec<Vec<RawProviderConfig>>) -> Vec<RawProviderConfig> {
    let mut order: Vec<String> = Vec::new();
    let mut by_id: std::collections::HashMap<String, RawProviderConfig> = std::collections::HashMap::new();

    for list in lists {
        for p in list {
            let Some(id) = p.id.clone() else { continue };
            match by_id.get_mut(&id) {
                Some(existing) => existing.merge_from(&p),
                None => {
                    order.push(id.clone());
                    by_id.insert(id, p);
                }
            }
        }
    }

    order.into_iter().filter_map(|id| by_id.remove(&id)).collect()
}

fn finalize_provider(raw: RawProviderConfig) -> Option<ProviderConfig> {
    Some(ProviderConfig {
        id: raw.id?,
        kind: raw.kind?,
        base_url: raw.base_url?,
        api_key: raw.api_key,
        label: raw.label,
    })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn load_config() -> LocalCodeConfig {
    load_config_for_cwd(&std::env::current_dir().unwrap_or_default())
}

pub fn load_config_for_cwd(cwd: &Path) -> LocalCodeConfig {
    let base = read_json::<RawConfig>(&config_path()).unwrap_or_default();
    let project = read_json::<RawConfig>(&project_config_path(cwd)).unwrap_or_default();

    let merged_providers = merge_providers(vec![
        default_providers(),
        base.providers.clone().unwrap_or_default(),
        project.providers.clone().unwrap_or_default(),
    ]);
    let providers: Vec<ProviderConfig> = merged_providers.into_iter().filter_map(finalize_provider).collect();

    LocalCodeConfig {
        providers,
        default_provider: project
            .default_provider
            .or(base.default_provider)
            .unwrap_or_else(|| "ollama".to_string()),
        default_model: project.default_model.or(base.default_model).unwrap_or_default(),
        auto_approve: project.auto_approve.or(base.auto_approve).unwrap_or(false),
    }
}

pub fn save_config(config: &LocalCodeConfig) -> std::io::Result<()> {
    let dir = config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let json = serde_json::to_string_pretty(config).unwrap_or_default();
    fs::write(config_path(), json + "\n")
}

pub fn get_config_path() -> PathBuf {
    config_path()
}
