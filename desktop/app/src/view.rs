use crate::app_state::{Message, Screen, State, TranscriptItem};
use crate::setup::{filtered_models, CustomProviderField, SetupMessage, SetupState};
use crate::theme;
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Font, Length};
use local_code_core::types::{LocalCodeConfig, ProviderType};

const SIDEBAR_WIDTH: f32 = 250.0;
const MONO: Font = Font::MONOSPACE;

pub fn view(state: &State) -> Element<'_, Message> {
    match &state.screen {
        Screen::Setup(setup_state) => container(setup_view(setup_state, &state.config))
            .padding(28)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::panel_container)
            .into(),
        Screen::Workspace => workspace_view(state),
    }
}

fn workspace_view(state: &State) -> Element<'_, Message> {
    row![sidebar_view(state), chat_panel_view(state)].into()
}

fn section_label(label: &str) -> Element<'static, Message> {
    text(label.to_string()).size(11).color(theme::palette().text_muted).into()
}

fn sidebar_view(state: &State) -> Element<'_, Message> {
    let mut col = column![
        text("LOCAL CODE").size(14).color(theme::palette().accent).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
        Space::new().height(20),
        section_label("PROJECTS"),
    ]
    .spacing(4)
    .width(Length::Fixed(SIDEBAR_WIDTH));

    for p in &state.workspace.projects {
        let active = state.workspace.active_project.as_deref() == Some(p.id.as_str());
        col = col.push(
            button(text(p.name.clone()).size(14))
                .on_press(Message::SelectProject(p.id.clone()))
                .width(Length::Fill)
                .padding([8, 10])
                .style(theme::sidebar_row(active)),
        );
    }
    col = col.push(
        button(text("+ Add project").size(13))
            .on_press_maybe(if state.busy { None } else { Some(Message::AddProjectPressed) })
            .width(Length::Fill)
            .padding([8, 10])
            .style(theme::ghost_button),
    );

    col = col.push(Space::new().height(20));
    col = col.push(section_label("CHATS"));

    if let Some(project_id) = state.workspace.active_project.clone() {
        for c in state.workspace.chats_for_project(&project_id) {
            let active = state.workspace.active_chat.as_deref() == Some(c.id.as_str());
            col = col.push(
                button(text(c.display_title()).size(14))
                    .on_press(Message::SelectChat(c.id.clone()))
                    .width(Length::Fill)
                    .padding([8, 10])
                    .style(theme::sidebar_row(active)),
            );
        }
        col = col.push(
            button(text("+ New chat").size(13))
                .on_press_maybe(if state.busy { None } else { Some(Message::NewChat) })
                .width(Length::Fill)
                .padding([8, 10])
                .style(theme::ghost_button),
        );
    } else {
        col = col.push(text("Add a project to start chatting.").size(12).color(theme::palette().text_muted));
    }

    container(scrollable(col.spacing(4)).height(Length::Fill))
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .padding(16)
        .style(theme::sidebar_container)
        .into()
}

fn chat_panel_view(state: &State) -> Element<'_, Message> {
    let Some(chat_id) = state.workspace.active_chat.clone() else {
        return container(text("Select or add a project to get started.").size(16).color(theme::palette().text_secondary))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(theme::panel_container)
            .into();
    };
    let chat_title = state.workspace.chat(&chat_id).map(|c| c.display_title()).unwrap_or_default();

    let header = row![
        text(chat_title).size(15).color(theme::palette().text_primary),
        Space::new().width(Length::Fill),
        checkbox(state.config.auto_approve).label("Auto-approve").on_toggle(Message::ToggleAutoApprove).size(14).text_size(13),
        button(text("Model").size(13))
            .on_press(Message::OpenProviderPicker)
            .padding([6, 12])
            .style(theme::ghost_button),
        button(text("Clear").size(13)).on_press(Message::ClearConversation).padding([6, 12]).style(theme::ghost_button),
        button(text(if state.show_help { "Hide help" } else { "Help" }).size(13))
            .on_press(Message::ShowHelp(!state.show_help))
            .padding([6, 12])
            .style(theme::ghost_button),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding([12, 20]);

    let mut body = column![].spacing(14).padding(iced::Padding { top: 8.0, right: 20.0, bottom: 20.0, left: 20.0 });

    if state.show_help {
        body = body.push(
            container(text(
                "read_file, write_file, edit_file, list_dir, glob, grep, bash are available to the model. \
Mutating tools ask for approval unless Auto-approve is on.",
            ).size(13).color(theme::palette().text_secondary))
            .padding(12)
            .width(Length::Fill)
            .style(theme::banner_container),
        );
    }

    if state.transcript.is_empty() {
        body = body.push(text("Send a message to get started.").size(13).color(theme::palette().text_muted));
    }

    for (idx, item) in state.transcript.iter().enumerate() {
        body = body.push(transcript_item_view(idx, item));
    }

    let transcript = scrollable(body).height(Length::Fill).width(Length::Fill);

    let input_row = row![
        text_input("Message Local Code\u{2026}", &state.input)
            .on_input(Message::InputChanged)
            .on_submit(Message::Submit)
            .padding([12, 18])
            .style(theme::input_field)
            .width(Length::Fill),
        button(text(if state.busy { "Working\u{2026}" } else { "Send" }).size(14))
            .on_press_maybe(if state.busy { None } else { Some(Message::Submit) })
            .padding([10, 18])
            .style(theme::primary_button),
    ]
    .spacing(10)
    .padding([16, 20])
    .align_y(Alignment::Center);

    container(column![header, transcript, input_row]).width(Length::Fill).height(Length::Fill).style(theme::panel_container).into()
}

fn transcript_item_view(idx: usize, item: &TranscriptItem) -> Element<'_, Message> {
    match item {
        TranscriptItem::User(text_content) => container(
            column![text("you").size(11).color(theme::palette().accent), text(text_content.clone()).size(14).color(theme::palette().text_primary)].spacing(6),
        )
        .padding(14)
        .width(Length::Fill)
        .style(theme::card_container)
        .into(),

        TranscriptItem::Assistant { text: text_content, thinking, thinking_expanded, .. } => {
            let mut col = column![text("assistant").size(11).color(theme::palette().text_secondary)].spacing(6);
            if !thinking.is_empty() {
                col = col.push(
                    button(text(if *thinking_expanded { "\u{25be} thinking" } else { "\u{25b8} thinking" }).size(12).color(theme::palette().text_muted))
                        .on_press(Message::ToggleThinking(idx))
                        .style(theme::ghost_button)
                        .padding([4, 8]),
                );
                if *thinking_expanded {
                    col = col.push(text(thinking.clone()).size(13).font(MONO).color(theme::palette().text_muted));
                }
            }
            if !text_content.is_empty() {
                col = col.push(text(text_content.clone()).size(14).color(theme::palette().text_primary));
            }
            container(col).padding(14).width(Length::Fill).style(theme::card_container).into()
        }

        TranscriptItem::Tool { name, args, output, approval } => {
            let is_error = output.as_ref().map(|(_, err)| *err).unwrap_or(false);
            let mut col = column![
                text(format!("\u{1f527} {name}")).size(12).color(theme::palette().accent),
                text(args_preview(args)).size(12).font(MONO).color(theme::palette().text_muted),
            ]
            .spacing(6);

            if let Some((preview, _responder)) = approval {
                col = col.push(text(preview.clone()).size(12).font(MONO).color(theme::palette().text_secondary));
                col = col.push(
                    row![
                        button(text("Approve").size(13)).on_press(Message::ApprovalChoice { approved: true, remember: false }).padding([6, 12]).style(theme::primary_button),
                        button(text("Approve, don't ask again").size(13))
                            .on_press(Message::ApprovalChoice { approved: true, remember: true })
                            .padding([6, 12])
                            .style(theme::ghost_button),
                        button(text("Deny").size(13)).on_press(Message::ApprovalChoice { approved: false, remember: false }).padding([6, 12]).style(theme::danger_button),
                    ]
                    .spacing(8),
                );
            }
            if let Some((output, is_err)) = output {
                let label = if *is_err { format!("error: {output}") } else { output.clone() };
                col = col.push(text(label).size(12).font(MONO).color(if *is_err { theme::palette().danger } else { theme::palette().text_secondary }));
            }

            let style = if is_error { theme::error_card_container } else { theme::card_container };
            container(col).padding(14).width(Length::Fill).style(style).into()
        }

        TranscriptItem::Notice(n) => {
            container(text(n.clone()).size(12).color(theme::palette().text_muted)).padding(10).width(Length::Fill).into()
        }
    }
}

fn args_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    serde_json::to_string(args).unwrap_or_default()
}

fn setup_view<'a>(setup_state: &'a SetupState, config: &'a LocalCodeConfig) -> Element<'a, Message> {
    let content: Element<'a, Message> = match setup_state {
        SetupState::Probing => text("Checking for available models...").size(15).color(theme::palette().text_secondary).into(),

        SetupState::Bootstrap { hw, recommended, alternatives } => {
            let mut col = column![
                text("Welcome to Local Code").size(24).color(theme::palette().text_primary),
                text(format!("Your system: ~{} GB RAM, {} CPU cores.", hw.total_ram_gb.round() as i64, hw.cpu_cores)).size(13).color(theme::palette().text_secondary),
                Space::new().height(8),
                text(format!(
                    "Recommended: {} (~{} GB) \u{2014} {}",
                    recommended.name, recommended.approx_size_gb, recommended.description
                ))
                .size(14)
                .color(theme::palette().text_primary),
                button(text(format!("Download {} (recommended)", recommended.name)).size(14))
                    .on_press(Message::Setup(SetupMessage::ChooseModel(recommended.name.to_string())))
                    .padding([10, 16])
                    .style(theme::primary_button),
            ]
            .spacing(10);
            for alt in alternatives {
                col = col.push(
                    button(text(format!("Download {} (~{} GB) \u{2014} {}", alt.name, alt.approx_size_gb, alt.description)).size(13))
                        .on_press(Message::Setup(SetupMessage::ChooseModel(alt.name.to_string())))
                        .padding([8, 14])
                        .style(theme::ghost_button),
                );
            }
            col = col.push(
                button(text("Skip \u{2014} I'll set this up manually").size(13))
                    .on_press(Message::Setup(SetupMessage::Skip))
                    .padding([8, 14])
                    .style(theme::ghost_button),
            );
            col.into()
        }

        SetupState::Downloading { model, log } => {
            let mut col = column![text(format!("Pulling {model}\u{2026}")).size(18).color(theme::palette().text_primary)].spacing(4);
            for line in log.iter().rev().take(20).rev() {
                col = col.push(text(line.clone()).size(12).font(MONO).color(theme::palette().text_secondary));
            }
            scrollable(col).height(Length::Fill).into()
        }

        SetupState::PickProvider { info } => {
            let mut col = column![text("Select a provider").size(20).color(theme::palette().text_primary)].spacing(8);
            if let Some(info) = info {
                col = col.push(text(info.clone()).size(13).color(theme::palette().warning));
            }
            for p in &config.providers {
                col = col.push(
                    button(text(format!("{}  ({})", p.display_label(), p.base_url)).size(14))
                        .on_press(Message::Setup(SetupMessage::SelectProvider(p.clone())))
                        .width(Length::Fill)
                        .padding(10)
                        .style(theme::ghost_button),
                );
            }
            col = col.push(
                button(text("+ Add a custom provider...").size(13))
                    .on_press(Message::Setup(SetupMessage::AddCustomPressed))
                    .padding(10)
                    .style(theme::ghost_button),
            );
            col.into()
        }

        SetupState::AddCustomProvider(form) => {
            let mut col = column![text("Add a custom provider").size(20).color(theme::palette().text_primary)].spacing(10);
            if let Some(err) = &form.error {
                col = col.push(text(err.clone()).size(13).color(theme::palette().danger));
            }
            col = col.push(
                row![
                    button(text("Ollama").size(13)).on_press(Message::Setup(SetupMessage::CustomKindChosen(ProviderType::Ollama))).padding([6, 12]).style(theme::ghost_button),
                    button(text("OpenAI-compatible").size(13))
                        .on_press(Message::Setup(SetupMessage::CustomKindChosen(ProviderType::OpenAiCompatible)))
                        .padding([6, 12])
                        .style(theme::ghost_button),
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
            )).size(13).color(theme::palette().text_secondary));
            col = col.push(
                text_input("Provider id (e.g. openai)", &form.id)
                    .padding(10)
                    .style(theme::input_field)
                    .on_input(|v| Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::Id, v))),
            );
            col = col.push(
                text_input("Base URL", &form.base_url)
                    .padding(10)
                    .style(theme::input_field)
                    .on_input(|v| Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::BaseUrl, v))),
            );
            col = col.push(
                text_input("Display label (optional)", &form.label)
                    .padding(10)
                    .style(theme::input_field)
                    .on_input(|v| Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::Label, v))),
            );
            col = col.push(
                text_input("API key (optional)", &form.api_key)
                    .padding(10)
                    .style(theme::input_field)
                    .secure(true)
                    .on_input(|v| Message::Setup(SetupMessage::CustomFieldChanged(CustomProviderField::ApiKey, v))),
            );
            col = col.push(
                row![
                    button(text("Add provider").size(14)).on_press(Message::Setup(SetupMessage::CustomSubmit)).padding([8, 14]).style(theme::primary_button),
                    button(text("Cancel").size(14)).on_press(Message::Setup(SetupMessage::CustomCancel)).padding([8, 14]).style(theme::ghost_button),
                ]
                .spacing(8),
            );
            col.into()
        }

        SetupState::PickModel { provider, models, filter, loading } => {
            let mut col = column![
                text(format!("Select a model ({})", provider.display_label())).size(20).color(theme::palette().text_primary),
                button(text("\u{2190} Back").size(13)).on_press(Message::Setup(SetupMessage::BackToProviders)).padding([6, 12]).style(theme::ghost_button),
            ]
            .spacing(8);
            if *loading {
                col = col.push(text("Loading models\u{2026}").size(13).color(theme::palette().text_secondary));
            } else {
                if models.len() > 8 {
                    col = col.push(
                        text_input("Filter\u{2026}", filter).padding(10).style(theme::input_field).on_input(|v| Message::Setup(SetupMessage::ModelFilterChanged(v))),
                    );
                }
                let matches = filtered_models(models, filter);
                let list = scrollable(matches.into_iter().fold(column![].spacing(4), |c, m| {
                    c.push(
                        button(text(m.to_string()).size(14))
                            .on_press(Message::Setup(SetupMessage::ModelSelected(m.to_string())))
                            .width(Length::Fill)
                            .padding(10)
                            .style(theme::ghost_button),
                    )
                }))
                .height(Length::Fill);
                col = col.push(list);
            }
            col.into()
        }

        SetupState::ManualModel { provider, value, info } => {
            let mut col = column![text(format!("Enter a model name for {}", provider.display_label())).size(20).color(theme::palette().text_primary)].spacing(10);
            if let Some(info) = info {
                col = col.push(text(info.clone()).size(13).color(theme::palette().warning));
            }
            col = col.push(
                text_input("Model name", value)
                    .padding(10)
                    .style(theme::input_field)
                    .on_input(|v| Message::Setup(SetupMessage::ManualModelChanged(v)))
                    .on_submit(Message::Setup(SetupMessage::ManualModelSubmit)),
            );
            col = col.push(
                row![
                    button(text("Continue").size(14)).on_press(Message::Setup(SetupMessage::ManualModelSubmit)).padding([8, 14]).style(theme::primary_button),
                    button(text("\u{2190} Back").size(14)).on_press(Message::Setup(SetupMessage::BackToProviders)).padding([8, 14]).style(theme::ghost_button),
                ]
                .spacing(8),
            );
            col.into()
        }

        SetupState::ConfirmSave { provider, model, save_default } => column![
            text(format!("Ready: {} / {}", provider.display_label(), model)).size(20).color(theme::palette().text_primary),
            checkbox(*save_default).label("Save as my default provider/model").on_toggle(|v| Message::Setup(SetupMessage::SaveDefaultToggled(v))),
            button(text("Start chatting").size(14)).on_press(Message::Setup(SetupMessage::Finish)).padding([10, 16]).style(theme::primary_button),
        ]
        .spacing(14)
        .into(),
    };

    content
}
