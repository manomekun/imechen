use gpui::{div, prelude::*, px, App, Div, ElementId, Stateful, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};

use super::tooltip;
use crate::theme::{Tokens, RADIUS_SM};

type Handler = Box<dyn Fn(&mut Window, &mut App) + 'static>;

/// Bordered button shell shared by the toolbar and the path picker.
pub(super) fn outlined_button(
    id: impl Into<ElementId>,
    t: &Tokens,
    disabled: bool,
) -> Stateful<Div> {
    let t = *t;
    div()
        .id(id)
        .h(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(t.border_strong)
        .rounded(px(RADIUS_SM))
        .bg(t.bg_field)
        .shadow(vec![t.shadow_sm()])
        .text_color(t.text_secondary)
        .when(disabled, |this| this.opacity(0.4))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.border_color(t.accent).text_color(t.accent))
        })
}

#[derive(IntoElement)]
pub struct Toolbar {
    tokens: Tokens,
    delete_disabled: bool,
    on_add: Option<Handler>,
    on_delete: Option<Handler>,
}

impl Toolbar {
    pub fn new(tokens: Tokens) -> Self {
        Self {
            tokens,
            delete_disabled: false,
            on_add: None,
            on_delete: None,
        }
    }

    pub fn delete_disabled(mut self, disabled: bool) -> Self {
        self.delete_disabled = disabled;
        self
    }

    pub fn on_add(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_add = Some(Box::new(handler));
        self
    }

    pub fn on_delete(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_delete = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let disabled = self.delete_disabled;

        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                outlined_button("toolbar-add", &t, false)
                    .px(px(18.))
                    .gap(px(8.))
                    .text_size(px(15.))
                    .text_color(t.text_primary)
                    .child(Icon::new(IconName::Plus).with_size(px(18.)))
                    .child("ファイルを追加")
                    .when_some(self.on_add, |this, handler| {
                        this.on_click(move |_, window, cx| handler(window, cx))
                    }),
            )
            .child(
                outlined_button("toolbar-delete", &t, disabled)
                    .w(px(40.))
                    .child(Icon::new(IconName::Trash).with_size(px(18.)))
                    .when(!disabled, |this| {
                        this.tooltip(tooltip("選択したファイルを削除"))
                            .when_some(self.on_delete, |this, handler| {
                                this.on_click(move |_, window, cx| handler(window, cx))
                            })
                    }),
            )
    }
}
