//! A dark, Cursor-like custom theme: layered near-black backgrounds, one accent color,
//! muted secondary text, subtle rounded borders — used everywhere in `view.rs` instead
//! of default widget styling.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme};

pub const BG_APP: Color = Color::from_rgb8(0x18, 0x18, 0x1c);
pub const BG_SIDEBAR: Color = Color::from_rgb8(0x14, 0x14, 0x17);
pub const BG_PANEL: Color = Color::from_rgb8(0x1b, 0x1b, 0x20);
pub const BG_CARD: Color = Color::from_rgb8(0x22, 0x22, 0x28);
pub const BG_ROW_ACTIVE: Color = Color::from_rgb8(0x2a, 0x2a, 0x38);
pub const BG_ROW_HOVER: Color = Color::from_rgb8(0x24, 0x24, 0x2c);

pub const ACCENT: Color = Color::from_rgb8(0x7c, 0x8c, 0xff);
pub const ACCENT_DIM: Color = Color::from_rgb8(0x4a, 0x52, 0x8a);

pub const TEXT_PRIMARY: Color = Color::from_rgb8(0xe8, 0xe8, 0xec);
pub const TEXT_SECONDARY: Color = Color::from_rgb8(0x93, 0x93, 0xa0);
pub const TEXT_MUTED: Color = Color::from_rgb8(0x63, 0x63, 0x70);

pub const BORDER: Color = Color::from_rgb8(0x2c, 0x2c, 0x34);
pub const SUCCESS: Color = Color::from_rgb8(0x5a, 0xc8, 0x8a);
pub const DANGER: Color = Color::from_rgb8(0xe0, 0x6c, 0x6c);
pub const WARNING: Color = Color::from_rgb8(0xe0, 0xb8, 0x6c);

pub fn theme() -> Theme {
    Theme::custom(
        "Local Code Dark".to_string(),
        iced::theme::Palette { background: BG_APP, text: TEXT_PRIMARY, primary: ACCENT, success: SUCCESS, warning: WARNING, danger: DANGER },
    )
}

fn no_shadow() -> Shadow {
    Shadow::default()
}

pub fn sidebar_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_SIDEBAR)),
        border: Border { color: BORDER, width: 1.0, radius: 0.0.into() },
        shadow: no_shadow(),
        snap: false,
    }
}

pub fn panel_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_PANEL)),
        border: Border::default(),
        shadow: no_shadow(),
        snap: false,
    }
}

pub fn card_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_CARD)),
        border: Border { color: BORDER, width: 1.0, radius: 10.0.into() },
        shadow: no_shadow(),
        snap: false,
    }
}

pub fn error_card_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_CARD)),
        border: Border { color: DANGER, width: 1.0, radius: 10.0.into() },
        shadow: no_shadow(),
        snap: false,
    }
}

pub fn banner_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SECONDARY),
        background: Some(Background::Color(ACCENT_DIM)),
        border: Border { color: BORDER, width: 1.0, radius: 8.0.into() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A project/chat row in the sidebar. `active` highlights the currently-selected item.
pub fn sidebar_row(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if active {
            BG_ROW_ACTIVE
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => BG_ROW_HOVER,
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border { color: if active { ACCENT } else { Color::TRANSPARENT }, width: if active { 1.0 } else { 0.0 }, radius: 8.0.into() },
            shadow: no_shadow(),
            snap: false,
        }
    }
}

pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = match status {
        button::Status::Hovered => Color::from_rgb8(0x8f, 0x9d, 0xff),
        button::Status::Pressed => ACCENT_DIM,
        button::Status::Disabled => Color::from_rgb8(0x3a, 0x3a, 0x46),
        button::Status::Active => ACCENT,
    };
    button::Style {
        background: Some(Background::Color(base)),
        text_color: Color::from_rgb8(0x10, 0x10, 0x14),
        border: Border { radius: 8.0.into(), ..Border::default() },
        shadow: no_shadow(),
        snap: false,
    }
}

pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => DANGER,
        _ => TEXT_SECONDARY,
    };
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color,
        border: Border { color: BORDER, width: 1.0, radius: 8.0.into() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A minimal outlined button for secondary actions (header icons, cancel, back).
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (BG_ROW_HOVER, TEXT_PRIMARY),
        button::Status::Pressed => (BG_ROW_ACTIVE, TEXT_PRIMARY),
        button::Status::Disabled => (Color::TRANSPARENT, TEXT_MUTED),
        button::Status::Active => (Color::TRANSPARENT, TEXT_SECONDARY),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border { color: BORDER, width: 1.0, radius: 8.0.into() },
        shadow: no_shadow(),
        snap: false,
    }
}
