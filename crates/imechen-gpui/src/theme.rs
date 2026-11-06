use std::sync::Arc;

use gpui::{
    linear_color_stop, linear_gradient, px, App, BoxShadow, Global, Hsla, Rgba, WindowAppearance,
};
use gpui_kit::component::{Theme, ThemeMode as KitThemeMode, ThemeToken};

use crate::settings::ThemeMode;

pub const BASE_FONT_SIZE: f32 = 13.0;
pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 12.0;

/// Design tokens from the UI spec. The full set lives here even though each
/// page reads only a subset.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    pub bg_base: Hsla,
    pub bg_sidebar: Hsla,
    pub bg_panel: Hsla,
    pub bg_footer: Hsla,
    pub bg_header: Hsla,
    pub bg_field: Hsla,
    pub bg_hover: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub track: Hsla,
    pub thumb_bg: Hsla,
    pub accent_tint: Hsla,
    pub accent_glow: Hsla,
    pub error_tint: Hsla,
    pub accent: Hsla,
    pub accent_light: Hsla,
    pub cta_from: Hsla,
    pub cta_to: Hsla,
    pub success: Hsla,
    pub error: Hsla,
    pub on_accent: Hsla,
}

impl Global for Tokens {}

fn hex(value: u32) -> Hsla {
    gpui::rgb(value).into()
}

/// CSS-style `rgba(r, g, b, a)` with 0-255 channels.
fn rgba(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a,
    }
    .into()
}

impl Tokens {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg_base: hex(0x1a1816),
            bg_sidebar: hex(0x131110),
            bg_panel: hex(0x201d1a),
            bg_footer: hex(0x1d1a18),
            bg_header: hex(0x25221f),
            bg_field: hex(0x2a2623),
            bg_hover: rgba(255, 255, 255, 0.05),
            border: rgba(255, 255, 255, 0.08),
            border_strong: rgba(255, 255, 255, 0.16),
            text_primary: hex(0xf3eee9),
            text_secondary: hex(0xaaa198),
            text_muted: hex(0x8a8279),
            track: rgba(255, 255, 255, 0.12),
            thumb_bg: hex(0x34302c),
            accent_tint: rgba(249, 115, 22, 0.16),
            accent_glow: rgba(249, 115, 22, 0.30),
            error_tint: rgba(239, 68, 68, 0.10),
            accent: hex(0xf97316),
            accent_light: hex(0xfb923c),
            cta_from: hex(0xf99a3c),
            cta_to: hex(0xef5f55),
            success: hex(0x22c55e),
            error: hex(0xef4444),
            on_accent: hex(0xffffff),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg_base: hex(0xffffff),
            bg_sidebar: hex(0xf4efeb),
            bg_panel: hex(0xfaf8f6),
            bg_footer: hex(0xf8f6f4),
            bg_header: hex(0xf5f3f1),
            bg_field: hex(0xffffff),
            bg_hover: rgba(60, 40, 20, 0.05),
            border: hex(0xe9e4df),
            border_strong: hex(0xd6cfc8),
            text_primary: hex(0x2a2521),
            text_secondary: hex(0x6e655d),
            text_muted: hex(0x8f867e),
            track: hex(0xe6e1dc),
            thumb_bg: hex(0xefeae5),
            accent_tint: rgba(249, 115, 22, 0.15),
            accent_glow: rgba(249, 115, 22, 0.25),
            error_tint: rgba(239, 68, 68, 0.06),
            ..Self::dark()
        }
    }

    #[allow(dead_code)]
    pub fn shadow_sm(&self) -> BoxShadow {
        BoxShadow {
            color: if self.dark {
                rgba(0, 0, 0, 0.4)
            } else {
                rgba(60, 40, 20, 0.08)
            },
            offset: gpui::point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: false,
        }
    }
}

pub trait ActiveTokens {
    fn tokens(&self) -> Tokens;
}

impl ActiveTokens for App {
    fn tokens(&self) -> Tokens {
        *self.global::<Tokens>()
    }
}

pub fn resolve_dark(mode: ThemeMode, appearance: WindowAppearance) -> bool {
    match mode {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
    }
}

/// Re-themes both our tokens and gpui-kit's global theme.
pub fn apply(mode: ThemeMode, appearance: WindowAppearance, cx: &mut App) {
    let tokens = if resolve_dark(mode, appearance) {
        Tokens::dark()
    } else {
        Tokens::light()
    };
    cx.set_global(tokens);

    // `change` reloads the registered kit theme over every color, so the
    // overrides must go in a separate `update` afterwards.
    let kit_mode = if tokens.dark {
        KitThemeMode::Dark
    } else {
        KitThemeMode::Light
    };
    Theme::change(kit_mode, None, cx);
    Theme::update(cx, |theme| map_onto_kit(theme, &tokens));
}

fn map_onto_kit(theme: &mut Theme, t: &Tokens) {
    theme.font_size = px(BASE_FONT_SIZE);
    theme.radius = px(RADIUS_SM);
    theme.radius_lg = px(RADIUS_MD);

    let c = &mut theme.colors;
    c.background = t.bg_base;
    c.foreground = t.text_primary;
    c.border = t.border;
    // Used as the border color of Input/Select/NumberInput.
    c.input = t.border_strong;
    c.ring = t.accent;
    c.caret = t.text_primary;
    c.selection = t.accent_tint;

    c.primary = t.accent;
    c.primary_hover = t.accent_light;
    c.primary_active = t.accent;
    c.primary_foreground = t.on_accent;
    c.button_primary = t.accent;
    c.button_primary_hover = t.accent_light;
    c.button_primary_active = t.accent;
    c.button_primary_foreground = t.on_accent;

    c.secondary = t.bg_field;
    c.secondary_hover = t.bg_hover;
    c.secondary_active = t.bg_hover;
    c.secondary_foreground = t.text_primary;
    c.muted = t.bg_hover;
    c.muted_foreground = t.text_secondary;
    // gpui-component uses `accent` as the hover/selected row background.
    c.accent = t.bg_hover;
    c.accent_foreground = t.text_primary;

    c.popover = t.bg_field;
    c.popover_foreground = t.text_primary;
    c.list = t.bg_field;
    c.list_hover = t.bg_hover;
    c.list_active = t.accent_tint;
    c.list_active_border = t.accent;

    c.danger = t.error;
    c.success = t.success;

    c.progress_bar = t.accent;
    c.slider_thumb = t.on_accent;
    c.scrollbar_thumb = t.border_strong;
    c.scrollbar_thumb_hover = t.text_muted;

    c.sidebar = t.bg_sidebar;
    c.sidebar_border = t.border;
    c.sidebar_foreground = t.text_primary;
    c.sidebar_accent = t.accent_tint;
    c.sidebar_accent_foreground = t.text_primary;

    theme.tokens.slider_bar = ThemeToken::new(
        t.accent,
        linear_gradient(
            90.,
            linear_color_stop(t.accent_light, 0.),
            linear_color_stop(t.accent, 1.),
        ),
    );

    // Input paints this instead of `input` mixed with transparency, which
    // would wash out the dark field background.
    let mut highlight = (*theme.highlight_theme).clone();
    highlight.style.editor_background = Some(t.bg_field);
    theme.highlight_theme = Arc::new(highlight);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_modes_ignore_the_os_appearance() {
        assert!(resolve_dark(ThemeMode::Dark, WindowAppearance::Light));
        assert!(!resolve_dark(ThemeMode::Light, WindowAppearance::Dark));
    }

    #[test]
    fn system_mode_follows_the_os_appearance() {
        assert!(resolve_dark(ThemeMode::System, WindowAppearance::Dark));
        assert!(resolve_dark(
            ThemeMode::System,
            WindowAppearance::VibrantDark
        ));
        assert!(!resolve_dark(ThemeMode::System, WindowAppearance::Light));
        assert!(!resolve_dark(
            ThemeMode::System,
            WindowAppearance::VibrantLight
        ));
    }

    #[test]
    fn dark_and_light_tokens_use_the_spec_surface_colors() {
        assert_eq!(Tokens::dark().bg_base, hex(0x1a1816));
        assert_eq!(Tokens::dark().bg_sidebar, hex(0x131110));
        assert_eq!(Tokens::light().bg_base, hex(0xffffff));
        assert_eq!(Tokens::light().bg_sidebar, hex(0xf4efeb));
        assert!(Tokens::dark().dark);
        assert!(!Tokens::light().dark);
    }

    #[test]
    fn accent_and_status_colors_are_shared_between_themes() {
        let (dark, light) = (Tokens::dark(), Tokens::light());
        assert_eq!(dark.accent, hex(0xf97316));
        assert_eq!(dark.accent, light.accent);
        assert_eq!(dark.error, light.error);
        assert_eq!(dark.success, light.success);
    }

    #[test]
    fn translucent_tokens_keep_their_spec_alpha() {
        assert_eq!(Tokens::dark().border.a, 0.08);
        assert_eq!(Tokens::dark().border_strong.a, 0.16);
        assert_eq!(Tokens::light().border.a, 1.0);
        assert_eq!(Tokens::light().accent_tint.a, 0.15);
    }
}
