//! A Cursor-like custom theme with matching light and dark palettes: layered translucent
//! backgrounds, one accent color, soft shadows instead of hard borders, and full-pill
//! buttons — used everywhere in `view.rs` instead of default widget styling.
//!
//! The active palette is picked once at startup from the OS preference (see `init`) and
//! held in a global; this app doesn't react live to the OS theme changing mid-session.

use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};
use std::sync::OnceLock;

pub struct Palette {
    pub bg_app: Color,
    pub bg_sidebar: Color,
    pub bg_panel: Color,
    pub bg_card: Color,
    pub bg_row_active: Color,
    pub bg_row_hover: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_dim: Color,
    pub accent_text: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub success: Color,
    pub danger: Color,
    pub warning: Color,
    pub shadow: Color,
}

const DARK: Palette = Palette {
    bg_app: Color::from_rgb8(0x17, 0x17, 0x1b),
    bg_sidebar: Color::from_rgb8(0x13, 0x13, 0x16),
    bg_panel: Color::from_rgb8(0x1a, 0x1a, 0x1f),
    bg_card: Color::from_rgb8(0x26, 0x26, 0x2e),
    bg_row_active: Color::from_rgb8(0x2d, 0x2d, 0x3d),
    bg_row_hover: Color::from_rgb8(0x23, 0x23, 0x2b),
    accent: Color::from_rgb8(0x7c, 0x8c, 0xff),
    accent_hover: Color::from_rgb8(0x8f, 0x9d, 0xff),
    accent_dim: Color::from_rgb8(0x4a, 0x52, 0x8a),
    accent_text: Color::from_rgb8(0x10, 0x10, 0x14),
    text_primary: Color::from_rgb8(0xe8, 0xe8, 0xec),
    text_secondary: Color::from_rgb8(0x93, 0x93, 0xa0),
    text_muted: Color::from_rgb8(0x63, 0x63, 0x70),
    success: Color::from_rgb8(0x5a, 0xc8, 0x8a),
    danger: Color::from_rgb8(0xe0, 0x6c, 0x6c),
    warning: Color::from_rgb8(0xe0, 0xb8, 0x6c),
    shadow: Color::BLACK,
};

const LIGHT: Palette = Palette {
    bg_app: Color::from_rgb8(0xf7, 0xf7, 0xf9),
    bg_sidebar: Color::from_rgb8(0xf0, 0xf0, 0xf3),
    bg_panel: Color::from_rgb8(0xfc, 0xfc, 0xfd),
    bg_card: Color::from_rgb8(0xff, 0xff, 0xff),
    bg_row_active: Color::from_rgb8(0xe6, 0xe8, 0xff),
    bg_row_hover: Color::from_rgb8(0xea, 0xea, 0xee),
    accent: Color::from_rgb8(0x5b, 0x5f, 0xe6),
    accent_hover: Color::from_rgb8(0x4a, 0x4e, 0xd6),
    accent_dim: Color::from_rgb8(0xd8, 0xda, 0xff),
    accent_text: Color::from_rgb8(0xff, 0xff, 0xff),
    text_primary: Color::from_rgb8(0x1c, 0x1c, 0x22),
    text_secondary: Color::from_rgb8(0x5a, 0x5a, 0x66),
    text_muted: Color::from_rgb8(0x9a, 0x9a, 0xa4),
    success: Color::from_rgb8(0x2f, 0x9e, 0x5f),
    danger: Color::from_rgb8(0xd6, 0x42, 0x42),
    warning: Color::from_rgb8(0xb8, 0x7c, 0x1a),
    shadow: Color::from_rgb8(0x8a, 0x8a, 0x9a),
};

static ACTIVE: OnceLock<&'static Palette> = OnceLock::new();

/// Picks the light or dark palette from the OS preference. Call once, before the first
/// `view()`/`theme()` call — subsequent calls are ignored.
pub fn init() {
    let prefers_dark = !matches!(dark_light::detect(), Ok(dark_light::Mode::Light));
    let _ = ACTIVE.set(if prefers_dark { &DARK } else { &LIGHT });
}

pub fn palette() -> &'static Palette {
    ACTIVE.get_or_init(|| &DARK)
}

/// A radius large enough to guarantee a full pill/stadium shape at any button height.
const PILL: f32 = 999.0;
const CARD_RADIUS: f32 = 16.0;

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn soft_shadow() -> Shadow {
    Shadow { color: alpha(palette().shadow, 0.22), offset: Vector::new(0.0, 2.0), blur_radius: 14.0 }
}

fn no_shadow() -> Shadow {
    Shadow::default()
}

pub fn theme() -> Theme {
    let p = palette();
    Theme::custom(
        "Local Code".to_string(),
        iced::theme::Palette { background: p.bg_app, text: p.text_primary, primary: p.accent, success: p.success, warning: p.warning, danger: p.danger },
    )
}

pub fn sidebar_container(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        text_color: Some(p.text_primary),
        background: Some(Background::Color(alpha(p.bg_sidebar, 0.96))),
        border: Border::default(),
        shadow: no_shadow(),
        snap: false,
    }
}

pub fn panel_container(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style { text_color: Some(p.text_primary), background: Some(Background::Color(p.bg_panel)), border: Border::default(), shadow: no_shadow(), snap: false }
}

/// Translucent, borderless, gently shadowed — the "glassy" surface for chat bubbles and
/// tool cards, layered over the panel background rather than boxed in with hard lines.
pub fn card_container(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        text_color: Some(p.text_primary),
        background: Some(Background::Color(alpha(p.bg_card, 0.82))),
        border: Border { radius: CARD_RADIUS.into(), ..Border::default() },
        shadow: soft_shadow(),
        snap: false,
    }
}

pub fn error_card_container(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        text_color: Some(p.text_primary),
        background: Some(Background::Color(alpha(p.bg_card, 0.82))),
        border: Border { color: alpha(p.danger, 0.5), width: 1.0, radius: CARD_RADIUS.into() },
        shadow: soft_shadow(),
        snap: false,
    }
}

pub fn banner_container(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        text_color: Some(p.text_secondary),
        background: Some(Background::Color(alpha(p.accent_dim, 0.6))),
        border: Border { radius: CARD_RADIUS.into(), ..Border::default() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A flat nav/list row (sidebar items, suggestion rows) — no outline, just a soft
/// highlight on hover/active, closer to Cursor's plain list style than a filled button.
pub fn nav_row(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let p = palette();
        let background = if active {
            alpha(p.bg_row_active, 0.75)
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => alpha(p.bg_row_hover, 0.7),
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: if active { p.text_primary } else { p.text_secondary },
            border: Border { radius: 10.0.into(), ..Border::default() },
            shadow: no_shadow(),
            snap: false,
        }
    }
}

/// A small icon-only affordance — the "+" next to a section header, a filter icon, etc.
pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => alpha(p.bg_row_hover, 0.9),
        _ => Color::TRANSPARENT,
    };
    button::Style { background: Some(Background::Color(background)), text_color: p.text_secondary, border: Border { radius: 8.0.into(), ..Border::default() }, shadow: no_shadow(), snap: false }
}

/// A borderless, backgroundless field — used inside a card that already supplies its own
/// pill shape (the empty-state composer), so the input itself shouldn't draw a second one.
pub fn bare_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let p = palette();
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: p.text_secondary,
        placeholder: p.text_muted,
        value: p.text_primary,
        selection: alpha(p.accent, 0.35),
    }
}

pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    let base = match status {
        button::Status::Hovered => p.accent_hover,
        button::Status::Pressed => p.accent_dim,
        button::Status::Disabled => alpha(p.text_muted, 0.4),
        button::Status::Active => p.accent,
    };
    button::Style {
        background: Some(Background::Color(base)),
        text_color: p.accent_text,
        border: Border { radius: PILL.into(), ..Border::default() },
        shadow: matches!(status, button::Status::Disabled)
            .then(no_shadow)
            .unwrap_or_else(|| Shadow { color: alpha(p.accent, 0.35), offset: Vector::new(0.0, 2.0), blur_radius: 10.0 }),
        snap: false,
    }
}

pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => p.danger,
        _ => p.text_secondary,
    };
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => alpha(p.danger, 0.14),
        _ => Color::TRANSPARENT,
    };
    button::Style { background: Some(Background::Color(background)), text_color, border: Border { radius: PILL.into(), ..Border::default() }, shadow: no_shadow(), snap: false }
}

/// A soft, stadium-rounded field for the message box and setup forms — no hard outline,
/// just a filled shape that brightens gently on focus.
pub fn input_field(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let p = palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(alpha(p.bg_card, if focused { 0.95 } else { 0.7 })),
        border: Border { color: if focused { alpha(p.accent, 0.6) } else { Color::TRANSPARENT }, width: 1.5, radius: 18.0.into() },
        icon: p.text_secondary,
        placeholder: p.text_muted,
        value: p.text_primary,
        selection: alpha(p.accent, 0.35),
    }
}

/// A round, outlined affordance for a composer toolbar (the "+" attach button) — a faint
/// ring that fills in gently on hover, sized as a circle rather than a rectangle.
pub fn composer_ring_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => alpha(p.bg_row_hover, 0.9),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: p.text_secondary,
        border: Border { color: alpha(p.text_muted, 0.45), width: 1.0, radius: PILL.into() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A solid, filled circle — the composer's mic/send button, high-contrast against the card
/// it sits on so it reads as the one primary action in the toolbar (mirrors Cursor's round
/// send button, which swaps between a mic glyph and an arrow depending on input state).
pub fn composer_send_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    let background = match status {
        button::Status::Hovered => alpha(p.text_primary, 0.85),
        button::Status::Pressed => alpha(p.text_primary, 0.7),
        button::Status::Disabled => alpha(p.text_muted, 0.3),
        button::Status::Active => p.text_primary,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: p.bg_panel,
        border: Border { radius: PILL.into(), ..Border::default() },
        shadow: no_shadow(),
        snap: false,
    }
}

/// A minimal filled pill for secondary actions (header icons, cancel, back).
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    let (background, text_color) = match status {
        button::Status::Hovered => (alpha(p.bg_row_hover, 0.9), p.text_primary),
        button::Status::Pressed => (alpha(p.bg_row_active, 0.9), p.text_primary),
        button::Status::Disabled => (alpha(p.bg_card, 0.4), p.text_muted),
        button::Status::Active => (alpha(p.bg_card, 0.6), p.text_secondary),
    };
    button::Style { background: Some(Background::Color(background)), text_color, border: Border { radius: PILL.into(), ..Border::default() }, shadow: no_shadow(), snap: false }
}
