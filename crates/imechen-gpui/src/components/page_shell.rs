use gpui::{div, prelude::*, px, AnyElement, App, FontWeight, ScrollHandle, SharedString, Window};

use super::Field;
use crate::pages::page_title;
use crate::theme::Tokens;

const PANEL_MIN_WIDTH: f32 = 260.0;
const PANEL_MAX_WIDTH: f32 = 300.0;

/// `clamp(260px, 25vw, 300px)`, with the window width standing in for `vw`.
fn panel_width(window_width: f32) -> f32 {
    (window_width * 0.25).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH)
}

#[derive(IntoElement)]
pub struct PageShell {
    title: SharedString,
    panel_title: SharedString,
    tokens: Tokens,
    scroll: ScrollHandle,
    notice: Option<AnyElement>,
    toolbar: Option<AnyElement>,
    fields: Vec<Field>,
    footer: Option<AnyElement>,
    body: Vec<AnyElement>,
}

impl PageShell {
    pub fn new(
        title: impl Into<SharedString>,
        panel_title: impl Into<SharedString>,
        scroll: &ScrollHandle,
        tokens: Tokens,
    ) -> Self {
        Self {
            title: title.into(),
            panel_title: panel_title.into(),
            tokens,
            scroll: scroll.clone(),
            notice: None,
            toolbar: None,
            fields: Vec::new(),
            footer: None,
            body: Vec::new(),
        }
    }

    pub fn notice(mut self, notice: Option<impl IntoElement>) -> Self {
        self.notice = notice.map(IntoElement::into_any_element);
        self
    }

    pub fn toolbar(mut self, toolbar: impl IntoElement) -> Self {
        self.toolbar = Some(toolbar.into_any_element());
        self
    }

    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl ParentElement for PageShell {
    /// Children fill the scrollable main area.
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for PageShell {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let panel_width = panel_width(f32::from(window.viewport_size().width));
        let last = self.fields.len().saturating_sub(1);

        let main = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(t.bg_base)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .gap(px(16.))
                    .pt(px(28.))
                    .px(px(24.))
                    .pb(px(16.))
                    .child(page_title(self.title, &t))
                    .children(self.notice)
                    .children(self.toolbar),
            )
            .child(
                div()
                    .id("page-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .px(px(24.))
                    .pb(px(24.))
                    .children(self.body),
            );

        let panel = div()
            .id("page-panel")
            .w(px(panel_width))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .pt(px(28.))
            .px(px(18.))
            .pb(px(16.))
            .bg(t.bg_panel)
            .border_l_1()
            .border_color(t.border)
            .child(
                div()
                    .pb(px(8.))
                    .text_size(px(20.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(t.text_primary)
                    .child(self.panel_title),
            )
            .children(
                self.fields
                    .into_iter()
                    .enumerate()
                    .map(|(i, field)| field.divider(i != last)),
            );

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().flex().child(main).child(panel))
            .children(self.footer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_windows_keep_the_minimum_panel_width() {
        assert_eq!(panel_width(960.), PANEL_MIN_WIDTH);
    }

    #[test]
    fn wide_windows_cap_the_panel_width() {
        assert_eq!(panel_width(1920.), PANEL_MAX_WIDTH);
    }

    #[test]
    fn mid_sized_windows_scale_the_panel_with_the_width() {
        assert_eq!(panel_width(1100.), 275.);
    }
}
