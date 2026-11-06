use gpui::{div, prelude::*, px, Context, FontWeight, SharedString, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};

use super::{page_title, Page};
use crate::settings::{self, ThemeMode};
use crate::theme::{self, ActiveTokens as _, Tokens, RADIUS_LG, RADIUS_MD};

const OPTIONS: [(ThemeMode, &str, IconName); 3] = [
    (ThemeMode::Dark, "ダーク", IconName::Moon),
    (ThemeMode::Light, "ライト", IconName::Sun),
    (ThemeMode::System, "システム", IconName::Monitor),
];

pub struct SettingsPage;

impl SettingsPage {
    fn select(&mut self, mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) {
        if settings::theme_mode(cx) == mode {
            return;
        }
        settings::save_theme_mode(mode);
        cx.set_global(settings::ActiveThemeMode(mode));
        theme::apply(mode, window.appearance(), cx);
    }

    fn option(
        &self,
        index: usize,
        (mode, label, icon): (ThemeMode, &'static str, IconName),
        t: Tokens,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = settings::theme_mode(cx) == mode;
        div()
            .id(("theme-option", index))
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.))
            .py(px(18.))
            .px(px(12.))
            .rounded(px(RADIUS_MD))
            .border_1()
            .cursor_pointer()
            .bg(if active { t.accent_tint } else { t.bg_field })
            .border_color(if active { t.accent } else { t.border_strong })
            .hover(|style| style.border_color(t.accent))
            .on_click(cx.listener(move |this, _, window, cx| this.select(mode, window, cx)))
            .child(Icon::new(icon).with_size(px(28.)).text_color(if active {
                t.accent
            } else {
                t.text_secondary
            }))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(if active {
                        t.text_primary
                    } else {
                        t.text_secondary
                    })
                    .when(active, |this| this.font_weight(FontWeight::SEMIBOLD))
                    .child(label),
            )
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let version: SharedString = format!("imechen v{}", env!("CARGO_PKG_VERSION")).into();

        div()
            .id("settings-page")
            .size_full()
            .overflow_y_scroll()
            .bg(t.bg_base)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(24.))
                    .py(px(28.))
                    .px(px(32.))
                    .child(page_title(Page::Settings.title(), &t))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .w_full()
                            .max_w(px(560.))
                            .p(px(20.))
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(RADIUS_LG))
                            .bg(t.bg_panel)
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("テーマ"),
                            )
                            .child(
                                div()
                                    .text_color(t.text_secondary)
                                    .child("アプリの外観テーマを選択してください"),
                            )
                            .child(
                                div().flex().gap(px(12.)).children(
                                    OPTIONS
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, option)| self.option(i, option, t, cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.text_muted)
                            .child(version),
                    ),
            )
    }
}
