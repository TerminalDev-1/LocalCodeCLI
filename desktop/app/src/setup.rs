//! First-run / no-model-configured wizard: tries to auto-bootstrap a small local Ollama
//! model if nothing at all is reachable, otherwise walks the user through picking a
//! provider and model, mirroring the original terminal `ui/setup.ts` flow.

use crate::app_state::Message;
use iced::Task;
use local_code_core::config::save_config;
use local_code_core::providers::registry::create_provider;
use local_code_core::system::hardware::{get_hardware_profile, recommend_model, HardwareProfile, ModelRecommendation};
use local_code_core::system::ollama::{
    is_ollama_installed, is_ollama_reachable, pull_model, start_ollama_server, wait_for_ollama, DEFAULT_WAIT_TIMEOUT_MS,
};
use local_code_core::types::{LocalCodeConfig, ProviderConfig, ProviderType};
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_UNFILTERED: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomProviderField {
    Id,
    BaseUrl,
    Label,
    ApiKey,
}

#[derive(Debug, Clone, Default)]
pub struct CustomProviderForm {
    pub kind: Option<ProviderType>,
    pub id: String,
    pub base_url: String,
    pub label: String,
    pub api_key: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SetupState {
    Probing,
    Bootstrap { hw: HardwareProfile, recommended: ModelRecommendation, alternatives: Vec<ModelRecommendation> },
    Downloading { model: String, log: Vec<String> },
    PickProvider { info: Option<String> },
    AddCustomProvider(CustomProviderForm),
    PickModel { provider: ProviderConfig, models: Vec<String>, filter: String, loading: bool },
    ManualModel { provider: ProviderConfig, value: String, info: Option<String> },
    ConfirmSave { provider: ProviderConfig, model: String, save_default: bool },
}

#[derive(Debug, Clone)]
pub enum SetupMessage {
    ProbeDone(bool),
    OllamaReachable(bool),
    OllamaInstalled(bool),
    OllamaServerReady(bool),
    ChooseModel(String),
    Skip,
    PullLine(String),
    PullDone(bool),
    SelectProvider(ProviderConfig),
    AddCustomPressed,
    CustomKindChosen(ProviderType),
    CustomFieldChanged(CustomProviderField, String),
    CustomSubmit,
    CustomCancel,
    ModelsLoaded(Result<Vec<String>, String>),
    ModelFilterChanged(String),
    ModelSelected(String),
    ManualModelChanged(String),
    ManualModelSubmit,
    BackToProviders,
    SaveDefaultToggled(bool),
    Finish,
}

pub enum SetupOutcome {
    Finished { provider: ProviderConfig, model: String },
}

pub fn probe_task(config: LocalCodeConfig) -> Task<Message> {
    Task::perform(probe_any_provider(config), |any| Message::Setup(SetupMessage::ProbeDone(any)))
}

async fn probe_any_provider(config: LocalCodeConfig) -> bool {
    let mut set = tokio::task::JoinSet::new();
    for p in &config.providers {
        let provider = create_provider(p);
        set.spawn(async move {
            tokio::time::timeout(Duration::from_secs(2), provider.list_models())
                .await
                .ok()
                .and_then(|r| r.ok())
                .map(|models| !models.is_empty())
                .unwrap_or(false)
        });
    }
    let mut any = false;
    while let Some(res) = set.join_next().await {
        if res.unwrap_or(false) {
            any = true;
        }
    }
    any
}

fn ollama_provider_config(config: &LocalCodeConfig) -> ProviderConfig {
    config.providers.iter().find(|p| p.kind == ProviderType::Ollama).cloned().unwrap_or(ProviderConfig {
        id: "ollama".to_string(),
        kind: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        label: Some("Ollama".to_string()),
    })
}

fn hardware_bootstrap_state() -> SetupState {
    let hw = get_hardware_profile();
    let rec = recommend_model(&hw);
    SetupState::Bootstrap { hw, recommended: rec.recommended, alternatives: rec.alternatives }
}

fn pull_model_task(model: String) -> Task<Message> {
    use futures::StreamExt;

    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let model_for_spawn = model.clone();
    let handle = tokio::spawn(async move { pull_model(&model_for_spawn, tx).await });

    let progress = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|line| (Message::Setup(SetupMessage::PullLine(line)), rx))
    });
    let done = futures::stream::once(async move {
        let ok = handle.await.unwrap_or(false);
        Message::Setup(SetupMessage::PullDone(ok))
    });

    Task::stream(progress.chain(done))
}

pub fn update(state: &mut SetupState, message: SetupMessage, config: &mut LocalCodeConfig) -> (Task<Message>, Option<SetupOutcome>) {
    match message {
        SetupMessage::ProbeDone(true) => {
            *state = SetupState::PickProvider { info: None };
            (Task::none(), None)
        }
        SetupMessage::ProbeDone(false) => {
            let base_url = ollama_provider_config(config).base_url;
            (Task::perform(is_ollama_reachable_owned(base_url), |r| Message::Setup(SetupMessage::OllamaReachable(r))), None)
        }
        SetupMessage::OllamaReachable(true) => {
            *state = hardware_bootstrap_state();
            (Task::none(), None)
        }
        SetupMessage::OllamaReachable(false) => {
            (Task::perform(is_ollama_installed(), |i| Message::Setup(SetupMessage::OllamaInstalled(i))), None)
        }
        SetupMessage::OllamaInstalled(false) => {
            *state = SetupState::PickProvider {
                info: Some(
                    "Ollama isn't installed. Install it from https://ollama.com/download, or set up a provider manually below."
                        .to_string(),
                ),
            };
            (Task::none(), None)
        }
        SetupMessage::OllamaInstalled(true) => {
            let base_url = ollama_provider_config(config).base_url;
            (Task::perform(start_and_wait_ollama(base_url), |ready| Message::Setup(SetupMessage::OllamaServerReady(ready))), None)
        }
        SetupMessage::OllamaServerReady(true) => {
            *state = hardware_bootstrap_state();
            (Task::none(), None)
        }
        SetupMessage::OllamaServerReady(false) => {
            *state = SetupState::PickProvider {
                info: Some("Couldn't reach Ollama after starting it. Try running `ollama serve` yourself.".to_string()),
            };
            (Task::none(), None)
        }
        SetupMessage::ChooseModel(model) => {
            *state = SetupState::Downloading { model: model.clone(), log: Vec::new() };
            (pull_model_task(model), None)
        }
        SetupMessage::Skip => {
            *state = SetupState::PickProvider { info: None };
            (Task::none(), None)
        }
        SetupMessage::PullLine(line) => {
            if let SetupState::Downloading { log, .. } = state {
                log.push(line);
            }
            (Task::none(), None)
        }
        SetupMessage::PullDone(true) => {
            let model = if let SetupState::Downloading { model, .. } = state { model.clone() } else { String::new() };
            let ollama = ollama_provider_config(config);
            if !config.providers.iter().any(|p| p.id == ollama.id) {
                config.providers.push(ollama.clone());
            }
            config.default_provider = ollama.id.clone();
            config.default_model = model.clone();
            let _ = save_config(config);
            (Task::none(), Some(SetupOutcome::Finished { provider: ollama, model }))
        }
        SetupMessage::PullDone(false) => {
            let model = if let SetupState::Downloading { model, .. } = state { model.clone() } else { String::new() };
            *state = SetupState::PickProvider {
                info: Some(format!("Failed to pull {model}. You can retry later with: ollama pull {model}")),
            };
            (Task::none(), None)
        }
        SetupMessage::SelectProvider(provider) => {
            *state = SetupState::PickModel { provider: provider.clone(), models: Vec::new(), filter: String::new(), loading: true };
            (list_models_task(provider), None)
        }
        SetupMessage::AddCustomPressed => {
            *state = SetupState::AddCustomProvider(CustomProviderForm::default());
            (Task::none(), None)
        }
        SetupMessage::CustomKindChosen(kind) => {
            if let SetupState::AddCustomProvider(form) = state {
                form.kind = Some(kind);
                if form.base_url.is_empty() {
                    form.base_url = match kind {
                        ProviderType::Ollama => "http://localhost:11434".to_string(),
                        ProviderType::OpenAiCompatible => "https://api.openai.com/v1".to_string(),
                    };
                }
            }
            (Task::none(), None)
        }
        SetupMessage::CustomFieldChanged(field, value) => {
            if let SetupState::AddCustomProvider(form) = state {
                match field {
                    CustomProviderField::Id => form.id = value,
                    CustomProviderField::BaseUrl => form.base_url = value,
                    CustomProviderField::Label => form.label = value,
                    CustomProviderField::ApiKey => form.api_key = value,
                }
            }
            (Task::none(), None)
        }
        SetupMessage::CustomCancel => {
            *state = SetupState::PickProvider { info: None };
            (Task::none(), None)
        }
        SetupMessage::CustomSubmit => {
            let SetupState::AddCustomProvider(form) = state else { return (Task::none(), None) };
            let Some(kind) = form.kind else {
                form.error = Some("Pick a provider type.".to_string());
                return (Task::none(), None);
            };
            if form.id.trim().is_empty() || form.base_url.trim().is_empty() {
                form.error = Some("Id and base URL are required.".to_string());
                return (Task::none(), None);
            }

            if let Some(existing) = config.providers.iter().find(|p| p.id == form.id) {
                let provider = existing.clone();
                *state = SetupState::PickModel { provider: provider.clone(), models: Vec::new(), filter: String::new(), loading: true };
                return (list_models_task(provider), None);
            }

            let provider = ProviderConfig {
                id: form.id.trim().to_string(),
                kind,
                base_url: form.base_url.trim().to_string(),
                api_key: if form.api_key.is_empty() { None } else { Some(form.api_key.clone()) },
                label: if form.label.is_empty() { None } else { Some(form.label.clone()) },
            };
            config.providers.push(provider.clone());
            *state = SetupState::PickModel { provider: provider.clone(), models: Vec::new(), filter: String::new(), loading: true };
            (list_models_task(provider), None)
        }
        SetupMessage::ModelsLoaded(result) => {
            let SetupState::PickModel { provider, .. } = state else { return (Task::none(), None) };
            let provider = provider.clone();
            match result {
                Ok(models) if !models.is_empty() => {
                    *state = SetupState::PickModel { provider, models, filter: String::new(), loading: false };
                }
                Ok(_) => {
                    *state = SetupState::ManualModel { provider, value: String::new(), info: None };
                }
                Err(e) => {
                    *state = SetupState::ManualModel { provider, value: String::new(), info: Some(format!("Could not reach provider: {e}")) };
                }
            }
            (Task::none(), None)
        }
        SetupMessage::ModelFilterChanged(text) => {
            if let SetupState::PickModel { filter, .. } = state {
                *filter = text;
            }
            (Task::none(), None)
        }
        SetupMessage::ModelSelected(model) => {
            let SetupState::PickModel { provider, .. } = state else { return (Task::none(), None) };
            *state = SetupState::ConfirmSave { provider: provider.clone(), model, save_default: true };
            (Task::none(), None)
        }
        SetupMessage::ManualModelChanged(text) => {
            if let SetupState::ManualModel { value, .. } = state {
                *value = text;
            }
            (Task::none(), None)
        }
        SetupMessage::ManualModelSubmit => {
            let SetupState::ManualModel { provider, value, .. } = state else { return (Task::none(), None) };
            if value.trim().is_empty() {
                return (Task::none(), None);
            }
            *state = SetupState::ConfirmSave { provider: provider.clone(), model: value.trim().to_string(), save_default: true };
            (Task::none(), None)
        }
        SetupMessage::BackToProviders => {
            *state = SetupState::PickProvider { info: None };
            (Task::none(), None)
        }
        SetupMessage::SaveDefaultToggled(v) => {
            if let SetupState::ConfirmSave { save_default, .. } = state {
                *save_default = v;
            }
            (Task::none(), None)
        }
        SetupMessage::Finish => {
            let SetupState::ConfirmSave { provider, model, save_default } = state else { return (Task::none(), None) };
            if *save_default {
                config.default_provider = provider.id.clone();
                config.default_model = model.clone();
                let _ = save_config(config);
            }
            (Task::none(), Some(SetupOutcome::Finished { provider: provider.clone(), model: model.clone() }))
        }
    }
}

fn list_models_task(provider: ProviderConfig) -> Task<Message> {
    Task::perform(
        async move {
            let p = create_provider(&provider);
            p.list_models().await.map_err(|e| e.to_string())
        },
        |r| Message::Setup(SetupMessage::ModelsLoaded(r)),
    )
}

async fn is_ollama_reachable_owned(base_url: String) -> bool {
    is_ollama_reachable(&base_url).await
}

async fn start_and_wait_ollama(base_url: String) -> bool {
    start_ollama_server().await;
    wait_for_ollama(&base_url, DEFAULT_WAIT_TIMEOUT_MS).await
}

pub fn filtered_models<'a>(models: &'a [String], filter: &str) -> Vec<&'a str> {
    if models.len() <= MAX_UNFILTERED || filter.is_empty() {
        return models.iter().map(|s| s.as_str()).collect();
    }
    let needle = filter.to_lowercase();
    models.iter().filter(|m| m.to_lowercase().contains(&needle)).map(|s| s.as_str()).collect()
}
