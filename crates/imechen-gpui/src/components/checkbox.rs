use gpui::{div, prelude::*, px, App, ElementId, SharedString, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};

use super::CheckState;
use crate::theme::Tokens;

type OnToggle = Box<dyn Fn(&mut Window, &mut App) + 'static>;

/// Drawn here because gpui-kit's checkbox has no indeterminate state and its
/// indicator is smaller than the spec's 20px box.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    state: CheckState,
    tokens: Tokens,
    label: Option<SharedString>,
    on_toggle: Option<OnToggle>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, state: CheckState, tokens: Tokens) -> Self {
        Self {
            id: id.into(),
            state,
            tokens,
            label: None,
            on_toggle: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_toggle(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let filled = self.state != CheckState::Unchecked;
        let mark = match self.state {
            CheckState::Unchecked => None,
            CheckState::Checked => Some(IconName::Check),
            CheckState::Indeterminate => Some(IconName::Minus),
        };

        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer()
            .child(
                div()
                    .size(px(20.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .border_1()
                    .border_color(if filled { t.accent } else { t.border_strong })
                    .bg(if filled { t.accent } else { t.bg_field })
                    .when(!filled, |this| this.hover(|s| s.border_color(t.accent)))
                    .children(
                        mark.map(|icon| Icon::new(icon).with_size(px(14.)).text_color(t.on_accent)),
                    ),
            )
            .children(self.label.map(|label| {
                div()
                    .text_size(px(15.))
                    .text_color(t.text_primary)
                    .child(label)
            }))
            .when_some(self.on_toggle, |this, handler| {
                this.on_click(move |_, window, cx| handler(window, cx))
            })
    }
}
