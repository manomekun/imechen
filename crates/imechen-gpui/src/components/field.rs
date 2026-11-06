use gpui::{div, prelude::*, px, AnyElement, App, FontWeight, SharedString, Window};

use crate::theme::{Tokens, RADIUS_SM};

#[derive(IntoElement)]
pub struct Field {
    label: SharedString,
    tokens: Tokens,
    aside: Option<AnyElement>,
    dimmed: bool,
    divider: bool,
    children: Vec<AnyElement>,
}

impl Field {
    pub fn new(label: impl Into<SharedString>, tokens: Tokens) -> Self {
        Self {
            label: label.into(),
            tokens,
            aside: None,
            dimmed: false,
            divider: true,
            children: Vec::new(),
        }
    }

    pub fn aside(mut self, aside: impl IntoElement) -> Self {
        self.aside = Some(aside.into_any_element());
        self
    }

    pub fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    /// Set by the panel, which knows which field is last.
    pub fn divider(mut self, divider: bool) -> Self {
        self.divider = divider;
        self
    }
}

impl ParentElement for Field {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Field {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .py(px(18.))
            .when(self.divider, |this| {
                this.border_b_1().border_color(t.border)
            })
            .when(self.dimmed, |this| this.opacity(0.45))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .min_h(px(28.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.text_primary)
                            .child(self.label),
                    )
                    .children(self.aside),
            )
            .children(self.children)
    }
}

#[derive(IntoElement)]
pub struct ValueBadge {
    text: SharedString,
    tokens: Tokens,
}

impl ValueBadge {
    pub fn new(text: impl Into<SharedString>, tokens: Tokens) -> Self {
        Self {
            text: text.into(),
            tokens,
        }
    }
}

impl RenderOnce for ValueBadge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        div()
            .min_w(px(64.))
            .h(px(36.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(t.border)
            .rounded(px(RADIUS_SM))
            .bg(t.bg_field)
            .text_size(px(15.))
            .text_color(t.text_primary)
            .child(self.text)
    }
}

#[derive(IntoElement)]
pub struct NoteText {
    text: SharedString,
    tokens: Tokens,
}

impl NoteText {
    pub fn new(text: impl Into<SharedString>, tokens: Tokens) -> Self {
        Self {
            text: text.into(),
            tokens,
        }
    }
}

impl RenderOnce for NoteText {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .text_size(px(12.))
            .text_color(self.tokens.text_muted)
            .child(self.text)
    }
}
