use crate::app_state::{Message, Screen, State, TranscriptItem};
use crate::setup::{filtered_models, CustomProviderField, SetupMessage, SetupState};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length};
use local_code_core::types::{LocalCodeConfig, ProviderType};

pub fn view(state: &State) -> Element<'_, Message> {
    match &state.screen {
        Screen::Setup(setup_state) => container(setup_view(setup_state, &state.config)).padding(24).into(),
        Screen::Chat => chat_view(state),
    }
}

fn setup_view<'a>(setup_state: &'a SetupState, config: &'a LocalCodeConfig) -> Element<'a, Message> {
    let content: Element<'a, Message> = match setup_state {
        SetupState::Probing => text("Checking for available models...").into(),

        SetupState::Bootstrap { hw, recommended, alternatives } => {
            let mut col = column![
                text("Welcome to Local Code").size(22),
                text(format!("Your system: ~{} GB RAM, {} CPU cores.", hw.total_ram_gb.round() as i64, hw.cpu_cores)),
                Space::new().height(8),
                text(format!(
                    "Recommended: {} (~{} GB) — {}",
                    recommended.name, recommended.approx_size_gb, recommended.description
                )),
                button(text(format!("Download {} (recommended)", recommended.name)))
                    .on_press(Message::Setup(SetupMessage::ChooseModel(recommended.name.to_string()))),
            ]
            .spacing(10);
            for alt in alternatives {
                col = col.push(
                    button(text(format!("Download {} (~{} GB) — {}", alt.name, alt.approx_size_gb, alt.description)))
                        .on_press(Message::Setup(SetupMessage::ChooseModel(alt.name.to_string()))),
                );
            }
            col = col.push(button(text("Skip — I'll set this up manually")).on_press(Message::Setup(SetupMessage::Skip)));
            col.into()
        }

        SetupState::Downloading { model, log } => {
            let mut col = column![text(format!("Pulling {model}\u{2026}")).size(18)].spacing(4);
            for line in log.iter().rev().take(20).rev() {
                col = col.push(text(line.clone()));
            }
            scrollable(col).height(Length::Fill).into()
        }

        SetupState::PickProvider { info } => {
            let mut col = column![text("Select a provider").size(20)].spacing(8);
            if let Some(info) = info {
                col = col.push(text(info.clone()));
            }
            for p in &config.providers {
                col = col.push(
                    button(text(format!("{}  ({})", p.display_label(), p.base_url)))
                        .on_press(Message::Setup(SetupMessage::SelectProvider(p.clone())))
                        .width(Length::Fill),
                );
            }
            col = col.push(button(text("+ Add a custom provider...")).on_press(Message::Setup(SetupMessage::AddCustomPressed)));
            col.into()
        }

        SetupState::AddCustomProvider(form) => {
            let mut col = column![text("Add a custom provider").size(20)].spacing(10);
            if let Some(err) = &form.error {
                col = col.push(text(err.clone()));
            }
            col = col.push(
                row![
                    button(text("Ollama")).on_press(Message::Setup(SetupMessage::CustomKindChosen(ProviderType::Ollama))),
                    button(text("OpenAI-compatible"))
                        .on_press(Message::Setup(SetupMessage::CustomKindChosen(ProviderType::OpenAiCompatible))),
                ]
                .spacing(8),
            );
            col = col.push(text(format!(
                "Type: {}",
                match form.kind {
                    Some(ProviderType::Ollama) => "Ollama",
                    Some(ProviderType::OpenAiCompatible) => "OpenAI-compatible",
                    None => "(pick one above)",
                }
            )));
            col = col.push(text_input("Provider id (e.g. openai)", &form.id).on_input(|v| {
                Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::Id, v))
            }));
            col = col.push(text_input("Base URL", &form.base_url).on_input(|v| {
                Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::BaseUrl, v))
            }));
            col = col.push(text_input("Display label (optional)", &form.label).on_input(|v| {
                Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::Label, v))
            }));
            col = col.push(text_input("API key (optional)", &form.api_key).secure(true).on_input(|v| {
                Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::ApiKey, v))
            }));
            col = col.push(
                row![
                    button(text("Add provider")).on_press(Message::Setup(SetupMessage::CustomSubmit)),
                    button(text("Cancel")).on_press(Message::Setup(SetupMessage::CustomCancel)),
                ]
                .spacing(8),
            );
            col.into()
        }

        SetupState::PickModel { provider, models, filter, loading } => {
            let mut col = column![
                text(format!("Select a model ({})", provider.display_label())).size(20),
                button(text("\u{2190} Back")).on_press(Message::Setup(SetupMessage::BackToProviders)),
            ]
            .spacing(8);
            if *loading {
                col = col.push(text("Loading models\u{2026}"));
            } else {
                if models.len() > 8 {
                    col = col.push(
                        text_input("Filter\u{2026}", filter)
                            .on_input(|v| Message::Setup(SetupMessage::ModelFilterChanged(v))),
                    );
                }
                let matches = filtered_models(models, filter);
                let list = scrollable(matches.into_iter().fold(column![].spacing(4), |c, m| {
                    c.push(button(text(m.to_string())).on_press(Message::Setup(SetupMessage::ModelSelected(m.to_string()))).width(Length::Fill))
                }))
                .height(Length::Fill);
                col = col.push(list);
            }
            col.into()
        }

        SetupState::ManualModel { provider, value, info } => {
            let mut col = column![text(format!("Enter a model name for {}", provider.display_label())).size(20)].spacing(10);
            if let Some(info) = info {
                col = col.push(text(info.clone()));
            }
            col = col.push(
                text_input("Model name", value)
                    .on_input(|v| Message::Setup(SetupMessage::ManualModelChanged(v)))
                    .on_submit(Message::Setup(SetupMessage::ManualModelSubmit)),
            );
            col = col.push(
                row![
                    button(text("Continue")).on_press(Message::Setup(SetupMessage::ManualModelSubmit)),
                    button(text("\u{2190} Back")).on_press(Message::Setup(SetupMessage::BackToProviders)),
                ]
                .spacing(8),
            );
            col.into()
        }

        SetupState::ConfirmSave { provider, model, save_default } => column![
            text(format!("Ready: {} / {}", provider.display_label(), model)).size(20),
            checkbox(*save_default)
                .label("Save as my default provider/model")
                .on_toggle(|v| Message::Setup(SetupMessage::SaveDefaultToggled(v))),
            button(text("Start chatting")).on_press(Message::Setup(SetupMessage::Finish)),
        ]
        .spacing(12)
        .into(),
    };

    content
}

fn chat_view(state: &State) -> Element<'_, Message> {
    let header = row![
        text(state.title()).size(16),
        Space::new().width(Length::Fill),
        checkbox(state.auto_approve).label("Auto-approve").on_toggle(Message::ToggleAutoApprove),
        button(text("Switch model/provider")).on_press(Message::OpenProviderPicker),
        button(text("Clear")).on_press(Message::ClearConversation),
        button(text(if state.show_help { "Hide help" } else { "Help" })).on_press(Message::ShowHelp(!state.show_help)),
    ]
    .spacing(10)
    .padding(10);

    let mut body = column![].spacing(14).padding(16);

    if state.show_help {
        body = body.push(container(text(
            "read_file, write_file, edit_file, list_dir, glob, grep, bash are available to the model. \
Mutating tools ask for approval unless Auto-approve is on. Use \"Switch model/provider\" to reconfigure at any time.",
        )));
    }

    for (idx, item) in state.transcript.iter().enumerate() {
        body = body.push(transcript_item_view(idx, item));
    }

    let transcript = scrollable(body).height(Length::Fill).width(Length::Fill);

    let input_row = row![
        text_input("Message Local Code\u{2026}", &state.input)
            .on_input(Message::InputChanged)
            .on_submit(Message::Submit)
            .width(Length::Fill),
        button(text("Send")).on_press_maybe(if state.busy { None } else { Some(Message::Submit) }),
    ]
    .spacing(8)
    .padding(10);

    column![header, transcript, input_row].into()
}

fn transcript_item_view(idx: usize, item: &TranscriptItem) -> Element<'_, Message> {
    match item {
        TranscriptItem::User(text_content) => {
            container(column![text("you").size(12), text(text_content.clone())].spacing(4)).padding(10).into()
        }
        TranscriptItem::Assistant { text: text_content, thinking, thinking_expanded, .. } => {
            let mut col = column![text("assistant").size(12)].spacing(4);
            if !thinking.is_empty() {
                col = col.push(
                    button(text(if *thinking_expanded { "\u{25be} thinking" } else { "\u{25b8} thinking" }))
                        .on_press(Message::ToggleThinking(idx)),
                );
                if *thinking_expanded {
                    col = col.push(text(thinking.clone()));
                }
            }
            if !text_content.is_empty() {
                col = col.push(text(text_content.clone()));
            }
            container(col).padding(10).into()
        }
        TranscriptItem::Tool { name, args, output, approval } => {
            let mut col = column![text(format!("\u{1f527} {name}")).size(13), text(args_preview(args))].spacing(4);
            if let Some((preview, _responder)) = approval {
                col = col.push(text(preview.clone()));
                col = col.push(
                    row![
                        button(text("Approve")).on_press(Message::ApprovalChoice { approved: true, remember: false }),
                        button(text("Approve, don't ask again")).on_press(Message::ApprovalChoice { approved: true, remember: true }),
                        button(text("Deny")).on_press(Message::ApprovalChoice { approved: false, remember: false }),
                    ]
                    .spacing(6),
                );
            }
            if let Some((output, is_error)) = output {
                col = col.push(text(if *is_error { format!("error: {output}") } else { output.clone() }));
            }
            container(col).padding(10).into()
        }
        TranscriptItem::Notice(n) => container(text(n.clone())).padding(10).into(),
    }
}

fn args_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    serde_json::to_string(args).unwrap_or_default()
}
