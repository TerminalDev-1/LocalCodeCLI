use crate::agent_bridge::{agent_event_stream, AppAgentEvent, ApprovalResponder, TurnRequest};
use crate::setup::{self, SetupMessage, SetupOutcome, SetupState};
use crate::workspace::{self, now_millis, Workspace};
use futures::StreamExt;
use iced::Task;
use local_code_core::agent::system_prompt::build_system_prompt;
use local_code_core::config::load_config;
use local_code_core::providers::registry::create_provider;
use local_code_core::tools::registry::all_tools;
use local_code_core::types::{LocalCodeConfig, Message as CoreMessage, Provider, ProviderConfig, Role};
use iced::window;
use iced::Subscription;
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub enum Screen {
    Setup(SetupState),
    Workspace,
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
    pub workspace: Workspace,
    pub provider: Option<Arc<dyn Provider>>,
    pub transcript: Vec<TranscriptItem>,
    pub input: String,
    pub busy: bool,
    pub session_auto_approve: bool,
    pub show_help: bool,
    pub window_id: Option<window::Id>,
    mid_session_setup: bool,
    last_picked: Option<(ProviderConfig, String)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Setup(SetupMessage),
    ConfigLoaded(LocalCodeConfig),
    InputChanged(String),
    UseSuggestion(String),
    Submit,
    Agent(AppAgentEvent),
    ApprovalChoice { approved: bool, remember: bool },
    ToggleThinking(usize),
    ToggleAutoApprove(bool),
    ClearConversation,
    OpenProviderPicker,
    ShowHelp(bool),
    AddProjectPressed,
    ProjectPicked(Option<PathBuf>),
    SelectProject(String),
    NewChat,
    SelectChat(String),
    WindowEvent(window::Id, window::Event),
    TitleBarDragged,
    WindowMinimize,
    WindowToggleMaximize,
    WindowClose,
    WindowDragResize(window::Direction),
}

impl State {
    pub fn new() -> (Self, Task<Message>) {
        let workspace = workspace::load_workspace();
        let state = Self {
            screen: Screen::Setup(SetupState::Probing),
            config: LocalCodeConfig { providers: vec![], default_provider: String::new(), default_model: String::new(), auto_approve: false },
            workspace,
            provider: None,
            transcript: Vec::new(),
            input: String::new(),
            busy: false,
            session_auto_approve: false,
            show_help: false,
            window_id: None,
            mid_session_setup: false,
            last_picked: None,
        };
        let task = Task::perform(async { load_config() }, Message::ConfigLoaded);
        (state, task)
    }

    pub fn title(&self) -> String {
        match self.workspace.active_project.as_deref().and_then(|id| self.workspace.project(id)) {
            Some(p) => format!("Local Code — {}", p.name),
            None => "Local Code".to_string(),
        }
    }

    /// Tracks the window's id (handed to us via the `Opened` event, since the default
    /// window an `iced::application` opens at startup doesn't hand back its id directly)
    /// so title-bar drag/minimize/maximize/close/resize can address it.
    pub fn subscription(&self) -> Subscription<Message> {
        window::events().map(|(id, event)| Message::WindowEvent(id, event))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigLoaded(config) => {
                self.config = config;
                if self.config.default_model.is_empty() {
                    self.enter_setup(false);
                    setup::probe_task(self.config.clone())
                } else {
                    self.enter_workspace();
                    Task::none()
                }
            }

            Message::Setup(m) => {
                let Screen::Setup(setup_state) = &mut self.screen else { return Task::none() };
                let (task, outcome) = setup::update(setup_state, m, &mut self.config);
                if let Some(SetupOutcome::Finished { provider, model }) = outcome {
                    self.last_picked = Some((provider.clone(), model.clone()));
                    if self.mid_session_setup {
                        self.mid_session_setup = false;
                        if let Some(chat_id) = self.workspace.active_chat.clone() {
                            if let Some(chat) = self.workspace.chat_mut(&chat_id) {
                                chat.provider_id = provider.id.clone();
                                chat.model = model;
                            }
                            let _ = workspace::save_workspace(&self.workspace);
                        }
                        self.provider = Some(create_provider(&provider));
                    }
                    self.screen = Screen::Workspace;
                }
                task
            }

            Message::InputChanged(text) => {
                self.input = text;
                Task::none()
            }

            Message::UseSuggestion(text) => {
                self.input = text;
                Task::none()
            }

            Message::Submit => {
                if self.busy || self.input.trim().is_empty() {
                    return Task::none();
                }
                let Some(chat_id) = self.workspace.active_chat.clone() else { return Task::none() };
                let text = std::mem::take(&mut self.input);
                self.transcript.push(TranscriptItem::User(text.clone()));
                if let Some(chat) = self.workspace.chat_mut(&chat_id) {
                    chat.messages.push(CoreMessage::user(text));
                    if chat.title.is_none() {
                        chat.title = Some(derive_title(&chat.messages));
                    }
                }
                let _ = workspace::save_workspace(&self.workspace);
                self.busy = true;
                self.start_turn(&chat_id)
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
                self.config.auto_approve = v;
                let _ = local_code_core::config::save_config(&self.config);
                Task::none()
            }

            Message::ClearConversation => {
                if let Some(chat_id) = self.workspace.active_chat.clone() {
                    self.seed_chat(&chat_id);
                    let _ = workspace::save_workspace(&self.workspace);
                }
                self.transcript.clear();
                Task::none()
            }

            Message::OpenProviderPicker => {
                if self.workspace.active_chat.is_none() || self.busy {
                    return Task::none();
                }
                self.enter_setup(true);
                Task::none()
            }

            Message::ShowHelp(v) => {
                self.show_help = v;
                Task::none()
            }

            Message::AddProjectPressed => {
                if self.busy {
                    return Task::none();
                }
                Task::perform(pick_folder(), Message::ProjectPicked)
            }

            Message::ProjectPicked(Some(path)) => {
                let project_id = self.workspace.add_project(path);
                let _ = workspace::save_workspace(&self.workspace);
                self.create_and_activate_chat(&project_id);
                Task::none()
            }
            Message::ProjectPicked(None) => Task::none(),

            Message::SelectProject(id) => {
                if self.busy {
                    return Task::none();
                }
                self.workspace.active_project = Some(id.clone());
                match self.workspace.most_recent_chat_for_project(&id).map(|c| c.id.clone()) {
                    Some(chat_id) => self.activate_chat(&chat_id),
                    None => {
                        self.workspace.active_chat = None;
                        self.transcript.clear();
                        self.provider = None;
                    }
                }
                let _ = workspace::save_workspace(&self.workspace);
                Task::none()
            }

            Message::NewChat => {
                if self.busy {
                    return Task::none();
                }
                let Some(project_id) = self.workspace.active_project.clone() else { return Task::none() };
                self.create_and_activate_chat(&project_id);
                Task::none()
            }

            Message::SelectChat(id) => {
                if self.busy {
                    return Task::none();
                }
                self.activate_chat(&id);
                Task::none()
            }

            Message::WindowEvent(id, event) => {
                if matches!(event, window::Event::Opened { .. }) {
                    self.window_id = Some(id);
                }
                Task::none()
            }
            Message::TitleBarDragged => self.window_id.map(window::drag).unwrap_or_else(Task::none),
            Message::WindowMinimize => self.window_id.map(|id| window::minimize(id, true)).unwrap_or_else(Task::none),
            Message::WindowToggleMaximize => self.window_id.map(window::toggle_maximize).unwrap_or_else(Task::none),
            Message::WindowClose => self.window_id.map(window::close).unwrap_or_else(Task::none),
            Message::WindowDragResize(direction) => self.window_id.map(|id| window::drag_resize(id, direction)).unwrap_or_else(Task::none),
        }
    }

    /// Enters the setup flow. On first run (`mid_session: false`) there's no config yet,
    /// so it starts by probing for a reachable provider. Mid-session (the composer's model
    /// picker), providers are already configured — jumping straight to `PickProvider` skips
    /// that probe, which used to leave the screen stuck on "Checking for available models..."
    /// forever, since nothing ever kicked off `probe_task` for this entry point.
    fn enter_setup(&mut self, mid_session: bool) {
        self.screen = Screen::Setup(if mid_session { SetupState::PickProvider { info: None } } else { SetupState::Probing });
        self.mid_session_setup = mid_session;
    }

    fn enter_workspace(&mut self) {
        self.screen = Screen::Workspace;
        if let Some(chat_id) = self.workspace.active_chat.clone() {
            if self.workspace.chat(&chat_id).is_some() {
                self.activate_chat(&chat_id);
                return;
            }
        }
        if let Some(project_id) = self.workspace.active_project.clone() {
            if let Some(chat_id) = self.workspace.most_recent_chat_for_project(&project_id).map(|c| c.id.clone()) {
                self.activate_chat(&chat_id);
            }
        }
    }

    fn create_and_activate_chat(&mut self, project_id: &str) {
        let (provider_id, model) = self.default_provider_and_model();
        let chat_id = self.workspace.add_chat(project_id, provider_id, model);
        self.seed_chat(&chat_id);
        let _ = workspace::save_workspace(&self.workspace);
        self.activate_chat(&chat_id);
    }

    fn default_provider_and_model(&self) -> (String, String) {
        if let Some((provider, model)) = &self.last_picked {
            return (provider.id.clone(), model.clone());
        }
        (self.config.default_provider.clone(), self.config.default_model.clone())
    }

    fn seed_chat(&mut self, chat_id: &str) {
        let Some(project_id) = self.workspace.chat(chat_id).map(|c| c.project_id.clone()) else { return };
        let Some(project) = self.workspace.project(&project_id) else { return };
        let tool_defs = all_tools().iter().map(|t| t.definition()).collect::<Vec<_>>();
        let prompt = build_system_prompt(&tool_defs, &project.path.to_string_lossy());
        if let Some(chat) = self.workspace.chat_mut(chat_id) {
            chat.messages = vec![CoreMessage::system(prompt)];
        }
    }

    fn activate_chat(&mut self, chat_id: &str) {
        self.workspace.active_chat = Some(chat_id.to_string());
        let Some(chat) = self.workspace.chat(chat_id) else { return };
        let project_id = chat.project_id.clone();
        let transcript = transcript_from_core_messages(&chat.messages);
        let provider = self.config.providers.iter().find(|p| p.id == chat.provider_id).map(create_provider);

        self.workspace.active_project = Some(project_id);
        self.transcript = transcript;
        self.provider = provider;
        self.session_auto_approve = false;
        self.busy = false;
        self.input.clear();
        let _ = workspace::save_workspace(&self.workspace);
    }

    fn start_turn(&mut self, chat_id: &str) -> Task<Message> {
        let Some(chat) = self.workspace.chat(chat_id) else {
            self.busy = false;
            return Task::none();
        };
        let Some(project) = self.workspace.project(&chat.project_id) else {
            self.busy = false;
            return Task::none();
        };
        let Some(provider_config) = self.config.providers.iter().find(|p| p.id == chat.provider_id) else {
            self.busy = false;
            self.transcript.push(TranscriptItem::Notice(format!("Unknown provider \"{}\" for this chat.", chat.provider_id)));
            return Task::none();
        };
        let req = TurnRequest {
            provider: create_provider(provider_config),
            model: chat.model.clone(),
            messages: chat.messages.clone(),
            cwd: project.path.clone(),
            auto_approve: self.config.auto_approve || self.session_auto_approve,
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
                self.busy = false;
                if let Some(TranscriptItem::Assistant { streaming, .. }) = self.transcript.last_mut() {
                    *streaming = false;
                }
                if let Some(chat_id) = self.workspace.active_chat.clone() {
                    if let Some(chat) = self.workspace.chat_mut(&chat_id) {
                        chat.messages = messages;
                        chat.updated_at = now_millis();
                    }
                    let _ = workspace::save_workspace(&self.workspace);
                }
                Task::none()
            }
        }
    }
}

fn derive_title(messages: &[CoreMessage]) -> String {
    let text = messages.iter().find(|m| m.role == Role::User).map(|m| m.content.as_str()).unwrap_or("New chat");
    let trimmed = text.trim();
    if trimmed.chars().count() > 48 {
        format!("{}\u{2026}", trimmed.chars().take(48).collect::<String>())
    } else if trimmed.is_empty() {
        "New chat".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Rebuilds the display transcript from a chat's canonical message history — used when a
/// persisted chat is opened. Reconstructed tool cards default to non-error styling since
/// success/failure isn't separately stored in the persisted history (minor, acceptable).
fn transcript_from_core_messages(messages: &[CoreMessage]) -> Vec<TranscriptItem> {
    let mut items = Vec::new();

    for m in messages {
        match m.role {
            Role::System => continue,
            Role::User => items.push(TranscriptItem::User(m.content.clone())),
            Role::Assistant => {
                if let Some(calls) = &m.tool_calls {
                    for tc in calls {
                        items.push(TranscriptItem::Tool { name: tc.name.clone(), args: tc.arguments.clone(), output: None, approval: None });
                    }
                }
                if !m.content.is_empty() {
                    items.push(TranscriptItem::Assistant {
                        text: m.content.clone(),
                        thinking: String::new(),
                        thinking_expanded: false,
                        streaming: false,
                    });
                }
            }
            Role::Tool => {
                if let Some(TranscriptItem::Tool { output, .. }) =
                    items.iter_mut().rev().find(|i| matches!(i, TranscriptItem::Tool { output: None, .. }))
                {
                    *output = Some((m.content.clone(), false));
                }
            }
        }
    }

    items
}

async fn pick_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new().pick_folder().await.map(|h| h.path().to_path_buf())
}
