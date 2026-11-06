use gpui::{div, linear_color_stop, linear_gradient, prelude::*, px, relative, App, Window};

use crate::theme::Tokens;

#[derive(IntoElement)]
pub struct ProgressBar {
    percent: u32,
    tokens: Tokens,
}

impl ProgressBar {
    pub fn new(percent: u32, tokens: Tokens) -> Self {
        Self { percent, tokens }
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        div()
            .w_full()
            .h(px(4.))
            .rounded_full()
            .bg(t.track)
            .overflow_hidden()
            .child(
                div()
                    .h_full()
                    .w(relative(self.percent.min(100) as f32 / 100.))
                    .rounded_full()
                    .bg(linear_gradient(
                        90.,
                        linear_color_stop(t.accent, 0.),
                        linear_color_stop(t.accent_light, 1.),
                    )),
            )
    }
}
