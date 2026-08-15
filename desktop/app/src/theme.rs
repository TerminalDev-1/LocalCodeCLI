//! A dark, Cursor-like custom theme: layered translucent backgrounds, one accent color,
//! muted secondary text, soft shadows instead of hard borders, and full-pill buttons —
//! used everywhere in `view.rs` instead of default widget styling.

use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

pub const BG_APP: Color = Color::from_rgb8(0x17, 0x17, 0x1b);
pub const BG_SIDEBAR: Color = Color::from_rgb8(0x13, 0x13, 0x16);
pub const BG_PANEL: Color = Color::from_rgb8(0x1a, 0x1a, 0x1f);
pub const BG_CARD: Color = Color::from_rgb8(0x26, 0x26, 0x2e);
pub const BG_ROW_ACTIVE: Color = Color::from_rgb8(0x2d, 0x2d, 0x3d);
pub const BG_ROW_HOVER: Color = Color::from_rgb8(0x23, 0x23, 0x2b);

pub const ACCENT: Color = Color::from_rgb8(0x7c, 0x8c, 0xff);
pub const ACCENT_DIM: Color = Color::from_rgb8(0x4a, 0x52, 0x8a);

pub const TEXT_PRIMARY: Color = Color::from_rgb8(0xe8, 0xe8, 0xec);
pub const TEXT_SECONDARY: Color = Color::from_rgb8(0x93, 0x93, 0xa0);
pub const TEXT_MUTED: Color = Color::from_rgb8(0x63, 0x63, 0x70);

pub const SUCCESS: Color = Color::from_rgb8(0x5a, 0xc8, 0x8a);
pub const DANGER: Color = Color::from_rgb8(0xe0, 0x6c, 0x6c);
pub const WARNING: Color = Color::from_rgb8(0xe0, 0xb8, 0x6c);

/// A radius large enough to guarantee a full pill/stadium shape at any button height.
const PILL: f32 = 999.0;
const CARD_RADIUS: f32 = 16.0;

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn soft_shadow() -> Shadow {
    Shadow { color: alpha(Color::BLACK, 0.28), offset: Vector::new(0.0, 2.0), blur_radius: 14.0 }
}

fn no_shadow() -> Shadow {
    Shadow::default()
}

pub fn theme() -> Theme {
    Theme::custom(
        "Local Code Dark".to_string(),
        iced::theme::Palette { background: BG_APP, text: TEXT_PRIMARY, primary: ACCENT, success: SUCCESS, warning: WARNING, danger: DANGER },
    )
}

pub fn sidebar_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(alpha(BG_SIDEBAR, 0.96))),
        border: Border::default(),
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

/// Translucent, borderless, gently shadowed — the "glassy" surface for chat bubbles and
/// tool cards, layered over the panel background rather than boxed in with hard lines.
pub fn card_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(alpha(BG_CARD, 0.72))),
        border: Border { radius: CARD_RADIUS.into(), ..Border::default() },
        shadow: soft_shadow(),
        snap: false,
    }
}

pub fn error_card_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(alpha(BG_CARD, 0.72))),
        border: Border { color: alpha(DANGER, 0.55), width: 1.0, radius: CARD_RADIUS.into() },
        shadow: soft_shadow(),
        snap: false,
    }
}

pub fn banner_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SECONDARY),
        background: Some(Background::Color(alpha(ACCENT_DIM, 0.55))),
        border: Border { radius: CARD_RADIUS.into(), ..Border::default() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A project/chat row in the sidebar. `active` highlights the currently-selected item.
pub fn sidebar_row(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if active {
            alpha(BG_ROW_ACTIVE, 0.9)
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => alpha(BG_ROW_HOVER, 0.85),
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border { radius: PILL.into(), ..Border::default() },
            shadow: no_shadow(),
            snap: false,
        }
    }
}

pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = match status {
        button::Status::Hovered => Color::from_rgb8(0x8f, 0x9d, 0xff),
        button::Status::Pressed => ACCENT_DIM,
        button::Status::Disabled => alpha(Color::from_rgb8(0x3a, 0x3a, 0x46), 0.6),
        button::Status::Active => ACCENT,
    };
    button::Style {
        background: Some(Background::Color(base)),
        text_color: Color::from_rgb8(0x10, 0x10, 0x14),
        border: Border { radius: PILL.into(), ..Border::default() },
        shadow: matches!(status, button::Status::Disabled).then(no_shadow).unwrap_or_else(|| Shadow {
            color: alpha(ACCENT, 0.35),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        }),
        snap: false,
    }
}

pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => DANGER,
        _ => TEXT_SECONDARY,
    };
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => alpha(DANGER, 0.14),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border { radius: PILL.into(), ..Border::default() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A soft, stadium-rounded field for the message box and setup forms — no hard outline,
/// just a filled shape that brightens gently on focus.
pub fn input_field(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(alpha(BG_CARD, if focused { 0.9 } else { 0.65 })),
        border: Border { color: if focused { alpha(ACCENT, 0.6) } else { Color::TRANSPARENT }, width: 1.5, radius: 18.0.into() },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_MUTED,
        value: TEXT_PRIMARY,
        selection: alpha(ACCENT, 0.35),
    }
}

/// A minimal filled pill for secondary actions (header icons, cancel, back).
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (alpha(BG_ROW_HOVER, 0.9), TEXT_PRIMARY),
        button::Status::Pressed => (alpha(BG_ROW_ACTIVE, 0.9), TEXT_PRIMARY),
        button::Status::Disabled => (alpha(BG_CARD, 0.4), TEXT_MUTED),
        button::Status::Active => (alpha(BG_CARD, 0.6), TEXT_SECONDARY),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border { radius: PILL.into(), ..Border::default() },
        shadow: no_shadow(),
        snap: false,
    }
}
