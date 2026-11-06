use std::path::PathBuf;

use gpui::{div, prelude::*, px, App, Div, FontWeight, SharedString, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};

use super::tooltip;
use crate::theme::{Tokens, RADIUS_MD, RADIUS_SM};

pub struct OutputItem {
    pub name: SharedString,
    pub detail: SharedString,
    pub path: PathBuf,
    pub full_path: Option<SharedString>,
}

pub struct ConvertError {
    pub name: SharedString,
    pub path: PathBuf,
    pub message: SharedString,
}

#[derive(IntoElement)]
pub struct ResultList {
    outputs: Vec<OutputItem>,
    errors: Vec<ConvertError>,
    tokens: Tokens,
}

impl ResultList {
    pub fn new(outputs: Vec<OutputItem>, errors: Vec<ConvertError>, tokens: Tokens) -> Self {
        Self {
            outputs,
            errors,
            tokens,
        }
    }
}

fn section_title(text: String, t: &Tokens) -> Div {
    div()
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(t.text_primary)
        .child(text)
}

fn item_shell(t: &Tokens, error: bool) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(12.))
        .py(px(10.))
        .px(px(14.))
        .border_1()
        .border_color(if error { t.error } else { t.border })
        .rounded(px(RADIUS_MD))
        .bg(if error { t.error_tint } else { t.bg_panel })
}

fn reveal_button(index: usize, path: PathBuf, t: &Tokens) -> impl IntoElement {
    let t = *t;
    div()
        .id(("reveal", index))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.))
        .py(px(6.))
        .px(px(10.))
        .border_1()
        .border_color(t.border_strong)
        .rounded(px(RADIUS_SM))
        .bg(t.bg_field)
        .text_size(px(12.))
        .text_color(t.accent)
        .cursor_pointer()
        .hover(|style| style.border_color(t.accent))
        .tooltip(tooltip("出力先を開く"))
        .on_click(move |_, _, cx| cx.reveal_path(&path))
        .child(Icon::new(IconName::ExternalLink).with_size(px(14.)))
        .child("出力先を開く")
}

impl RenderOnce for ResultList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let mono = cx.theme().mono_font_family.clone();

        let outputs = (!self.outputs.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .mt(px(24.))
                .child(section_title(format!("出力 ({})", self.outputs.len()), &t))
                .children(self.outputs.into_iter().enumerate().map(|(i, item)| {
                    item_shell(&t, false)
                        .items_center()
                        .child(
                            Icon::new(IconName::CircleCheck)
                                .with_size(px(18.))
                                .text_color(t.success),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .truncate()
                                        .child(item.name),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(t.text_secondary)
                                        .child(item.detail),
                                )
                                .children(item.full_path.map(|full| {
                                    div()
                                        .id(("full-path", i))
                                        .text_size(px(11.))
                                        .font_family(mono.clone())
                                        .text_color(t.text_muted)
                                        .truncate()
                                        .tooltip(tooltip(full.clone()))
                                        .child(full)
                                })),
                        )
                        .child(reveal_button(i, item.path, &t))
                }))
        });

        let errors = (!self.errors.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .mt(px(24.))
                .child(section_title(format!("エラー ({})", self.errors.len()), &t))
                .children(self.errors.into_iter().enumerate().map(|(i, err)| {
                    item_shell(&t, true)
                        .child(
                            Icon::new(IconName::CircleAlert)
                                .with_size(px(18.))
                                .text_color(t.error),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .id(("error-name", i))
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .truncate()
                                        .tooltip(tooltip(err.path.display().to_string()))
                                        .child(err.name),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(t.error)
                                        .child(err.message),
                                ),
                        )
                }))
        });

        div().children(outputs).children(errors)
    }
}
