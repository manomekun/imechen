mod animation;
mod image;
mod settings;
mod video;

pub use animation::AnimationPage;
pub use image::ImagePage;
pub use settings::SettingsPage;
pub use video::VideoPage;

use gpui::{div, prelude::*, px, Div, FontWeight, SharedString};
use gpui_kit::assets::IconName;

use crate::theme::Tokens;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Image,
    Video,
    Animation,
    Settings,
}

impl Page {
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "画像変換",
            Self::Video => "動画変換",
            Self::Animation => "アニメーション",
            Self::Settings => "設定",
        }
    }

    /// The animation page is titled differently from its nav label.
    pub fn title(self) -> &'static str {
        match self {
            Self::Animation => "アニメーション作成",
            other => other.label(),
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Image => IconName::Image,
            Self::Video => IconName::Video,
            Self::Animation => IconName::Layers,
            Self::Settings => IconName::Settings,
        }
    }
}

pub fn page_title(title: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .text_size(px(28.))
        .font_weight(FontWeight::BOLD)
        .text_color(tokens.text_primary)
        .child(title.into())
}

/// Placeholder body shared by pages whose content has not been built yet.
pub fn title_only(page: Page, tokens: &Tokens) -> Div {
    div()
        .size_full()
        .bg(tokens.bg_base)
        .pt(px(28.))
        .px(px(24.))
        .pb(px(16.))
        .child(page_title(page.title(), tokens))
}
