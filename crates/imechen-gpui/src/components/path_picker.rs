use std::path::Path;

use gpui::{div, prelude::*, px, App, SharedString, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};

use super::toolbar::outlined_button;
use super::tooltip;
use crate::theme::{Tokens, RADIUS_SM};

type OnPick = Box<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct PathPicker {
    path: SharedString,
    tokens: Tokens,
    on_pick: Option<OnPick>,
}

impl PathPicker {
    pub fn new(path: &Path, tokens: Tokens) -> Self {
        Self {
            path: path.display().to_string().into(),
            tokens,
            on_pick: None,
        }
    }

    pub fn on_pick(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for PathPicker {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let on_pick = self.on_pick.map(std::rc::Rc::new);
        let click = |on_pick: Option<std::rc::Rc<OnPick>>| {
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                if let Some(handler) = &on_pick {
                    handler(window, cx);
                }
            }
        };

        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .id("path-box")
                    .flex_1()
                    .min_w_0()
                    .h(px(40.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(t.border_strong)
                    .rounded(px(RADIUS_SM))
                    .bg(t.bg_field)
                    .text_size(px(15.))
                    .text_color(t.text_primary)
                    .cursor_pointer()
                    .hover(|style| style.border_color(t.accent).text_color(t.accent))
                    .tooltip(tooltip(self.path.clone()))
                    .on_click(click(on_pick.clone()))
                    .child(div().min_w_0().truncate().child(self.path)),
            )
            .child(
                outlined_button("path-pick", &t, false)
                    .size(px(40.))
                    .flex_shrink_0()
                    .child(Icon::new(IconName::Folder).with_size(px(18.)))
                    .tooltip(tooltip("保存先を選択"))
                    .on_click(click(on_pick)),
            )
    }
}
