use crate::agent_bridge::{agent_event_stream, AppAgentEvent, ApprovalResponder, TurnRequest};
use crate::setup::{self, SetupMessage, SetupOutcome, SetupState};
use futures::StreamExt;
use iced::Task;
use local_code_core::agent::system_prompt::build_system_prompt;
use local_code_core::config::{load_config, save_config};
use local_code_core::providers::registry::{create_provider, resolve_provider};
use local_code_core::tools::registry::all_tools;
use local_code_core::types::{LocalCodeConfig, Message as CoreMessage, Provider, ProviderConfig};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub enum Screen {
    Setup(SetupState),
    Chat,
}

#[derive(Debug, Clone)]
pub enum TranscriptItem {
    User(String),
    Assistant { text: String, thinking: String, thinking_expanded: bool, streaming: bool },
    Tool { name: String, args: Map<String, Value>, output: Option<(String, bool)>, approval: Option<(String, ApprovalResponder)> },
    Notice(String),
}

pub struct State {
    pub screen: Screen,
    pub config: LocalCodeConfig,
    pub provider: Option<Arc<dyn Provider>>,
    pub provider_config: Option<ProviderConfig>,
    pub model: String,
    pub cwd: PathBuf,
    pub auto_approve: bool,
    pub session_auto_approve: bool,
    pub core_messages: Vec<CoreMessage>,
    pub transcript: Vec<TranscriptItem>,
    pub input: String,
    pub busy: bool,
    pub show_help: bool,
    mid_session_setup: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Setup(SetupMessage),
    ConfigLoaded(LocalCodeConfig),
    InputChanged(String),
    Submit,
    Agent(AppAgentEvent),
    ApprovalChoice { approved: bool, remember: bool },
    ToggleThinking(usize),
    ToggleAutoApprove(bool),
    ClearConversation,
    OpenProviderPicker,
    ShowHelp(bool),
}

impl State {
    pub fn new() -> (Self, Task<Message>) {
        let state = Self {
            screen: Screen::Setup(SetupState::Probing),
            config: LocalCodeConfig { providers: vec![], default_provider: String::new(), default_model: String::new(), auto_approve: false },
            provider: None,
            provider_config: None,
            model: String::new(),
            cwd: std::env::current_dir().unwrap_or_default(),
            auto_approve: false,
            session_auto_approve: false,
            core_messages: Vec::new(),
            transcript: Vec::new(),
            input: String::new(),
            busy: false,
            show_help: false,
            mid_session_setup: false,
        };
        let task = Task::perform(async { load_config() }, Message::ConfigLoaded);
        (state, task)
    }

    pub fn title(&self) -> String {
        match &self.provider_config {
            Some(p) if !self.model.is_empty() => format!("Local Code — {} / {}", p.display_label(), self.model),
            _ => "Local Code".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigLoaded(config) => {
                self.auto_approve = config.auto_approve;
                self.config = config;
                if self.config.default_model.is_empty() {
                    self.enter_setup(false);
                    Task::batch([setup::probe_task(self.config.clone())])
                } else {
                    match resolve_provider(&self.config, None) {
                        Ok(provider) => {
                            let provider_config = self.config.providers.iter().find(|p| p.id == self.config.default_provider).cloned();
                            self.provider = Some(provider);
                            self.provider_config = provider_config;
                            self.model = self.config.default_model.clone();
                            self.enter_chat_fresh();
                            Task::none()
                        }
                        Err(_) => {
                            self.enter_setup(false);
                            Task::batch([setup::probe_task(self.config.clone())])
                        }
                    }
                }
            }

            Message::Setup(m) => {
                let Screen::Setup(setup_state) = &mut self.screen else { return Task::none() };
                let (task, outcome) = setup::update(setup_state, m, &mut self.config);
                if let Some(SetupOutcome::Finished { provider, model }) = outcome {
                    self.provider = Some(create_provider(&provider));
                    self.provider_config = Some(provider);
                    self.model = model;
                    if self.mid_session_setup {
                        self.mid_session_setup = false;
                        self.screen = Screen::Chat;
                    } else {
                        self.enter_chat_fresh();
                    }
                }
                task
            }

            Message::InputChanged(text) => {
                self.input = text;
                Task::none()
            }

            Message::Submit => {
                if self.busy || self.input.trim().is_empty() {
                    return Task::none();
                }
                let text = std::mem::take(&mut self.input);
                self.transcript.push(TranscriptItem::User(text.clone()));
                self.core_messages.push(CoreMessage::user(text));
                self.busy = true;
                self.start_turn()
            }

            Message::Agent(event) => self.handle_agent_event(event),

            Message::ApprovalChoice { approved, remember } => {
                if remember && approved {
                    self.session_auto_approve = true;
                }
                if let Some(TranscriptItem::Tool { approval, .. }) =
                    self.transcript.iter_mut().rev().find(|i| matches!(i, TranscriptItem::Tool { approval: Some(_), .. }))
                {
                    if let Some((_, responder)) = approval.take() {
                        responder.respond(approved);
                    }
                }
                Task::none()
            }

            Message::ToggleThinking(idx) => {
                if let Some(TranscriptItem::Assistant { thinking_expanded, .. }) = self.transcript.get_mut(idx) {
                    *thinking_expanded = !*thinking_expanded;
                }
                Task::none()
            }

            Message::ToggleAutoApprove(v) => {
                self.auto_approve = v;
                self.config.auto_approve = v;
                let _ = save_config(&self.config);
                Task::none()
            }

            Message::ClearConversation => {
                self.transcript.clear();
                self.seed_system_message();
                Task::none()
            }

            Message::OpenProviderPicker => {
                self.enter_setup(true);
                Task::none()
            }

            Message::ShowHelp(v) => {
                self.show_help = v;
                Task::none()
            }
        }
    }

    fn enter_setup(&mut self, mid_session: bool) {
        let (state, _task) = setup::start();
        self.screen = Screen::Setup(state);
        self.mid_session_setup = mid_session;
    }

    fn seed_system_message(&mut self) {
        let tool_defs = all_tools().iter().map(|t| t.definition()).collect::<Vec<_>>();
        let prompt = build_system_prompt(&tool_defs, &self.cwd.to_string_lossy());
        self.core_messages = vec![CoreMessage::system(prompt)];
    }

    fn enter_chat_fresh(&mut self) {
        self.seed_system_message();
        self.transcript.clear();
        self.session_auto_approve = false;
        self.screen = Screen::Chat;
    }

    fn start_turn(&mut self) -> Task<Message> {
        let Some(provider) = self.provider.clone() else {
            self.busy = false;
            return Task::none();
        };
        let req = TurnRequest {
            provider,
            model: self.model.clone(),
            messages: self.core_messages.clone(),
            cwd: self.cwd.clone(),
            auto_approve: self.auto_approve || self.session_auto_approve,
        };
        Task::stream(agent_event_stream(req).map(Message::Agent))
    }

    fn last_pending_tool_mut(&mut self) -> Option<&mut TranscriptItem> {
        self.transcript.iter_mut().rev().find(|i| matches!(i, TranscriptItem::Tool { output: None, .. }))
    }

    fn handle_agent_event(&mut self, event: AppAgentEvent) -> Task<Message> {
        match event {
            AppAgentEvent::TextChunk(t) => {
                if let Some(TranscriptItem::Assistant { text, streaming, .. }) = self.transcript.last_mut() {
                    if *streaming {
                        text.push_str(&t);
                        return Task::none();
                    }
                }
                self.transcript.push(TranscriptItem::Assistant {
                    text: t,
                    thinking: String::new(),
                    thinking_expanded: false,
                    streaming: true,
                });
                Task::none()
            }
            AppAgentEvent::ThinkingChunk(t) => {
                if let Some(TranscriptItem::Assistant { thinking, streaming, .. }) = self.transcript.last_mut() {
                    if *streaming {
                        thinking.push_str(&t);
                        return Task::none();
                    }
                }
                self.transcript.push(TranscriptItem::Assistant {
                    text: String::new(),
                    thinking: t,
                    thinking_expanded: false,
                    streaming: true,
                });
                Task::none()
            }
            AppAgentEvent::ToolStart { name, args } => {
                self.transcript.push(TranscriptItem::Tool { name, args, output: None, approval: None });
                Task::none()
            }
            AppAgentEvent::NeedsApproval { preview, respond } => {
                if let Some(TranscriptItem::Tool { approval, .. }) = self.last_pending_tool_mut() {
                    *approval = Some((preview, respond));
                }
                Task::none()
            }
            AppAgentEvent::ToolResult { output, is_error } => {
                if let Some(TranscriptItem::Tool { output: out, approval, .. }) = self.last_pending_tool_mut() {
                    *out = Some((output, is_error));
                    *approval = None;
                }
                Task::none()
            }
            AppAgentEvent::Notice(n) => {
                self.transcript.push(TranscriptItem::Notice(n));
                Task::none()
            }
            AppAgentEvent::Done { messages } => {
                self.core_messages = messages;
                self.busy = false;
                if let Some(TranscriptItem::Assistant { streaming, .. }) = self.transcript.last_mut() {
                    *streaming = false;
                }
                Task::none()
            }
        }
    }
}
