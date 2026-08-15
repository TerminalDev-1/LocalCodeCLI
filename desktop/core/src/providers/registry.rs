use crate::types::{LocalCodeConfig, Provider, ProviderConfig, ProviderType};
use std::sync::Arc;

pub fn create_provider(config: &ProviderConfig) -> Arc<dyn Provider> {
    match config.kind {
        ProviderType::Ollama => Arc::new(super::ollama::OllamaProvider::new(config)),
        ProviderType::OpenAiCompatible => Arc::new(super::openai_compatible::OpenAiCompatibleProvider::new(config)),
    }
}

pub fn resolve_provider(config: &LocalCodeConfig, provider_id: Option<&str>) -> Result<Arc<dyn Provider>, String> {
    let id = provider_id.unwrap_or(&config.default_provider);
    let provider_config = config.providers.iter().find(|p| p.id == id).ok_or_else(|| {
        let known = config.providers.iter().map(|p| p.id.clone()).collect::<Vec<_>>().join(", ");
        format!("Unknown provider \"{id}\". Configured providers: {known}")
    })?;
    Ok(create_provider(provider_config))
}

pub fn list_providers(config: &LocalCodeConfig) -> &[ProviderConfig] {
    &config.providers
}
