use gpui::{
    div, linear_color_stop, linear_gradient, point, prelude::*, px, App, BoxShadow, ElementId,
    FontWeight, Hsla, SharedString, Window,
};

use crate::theme::{Tokens, RADIUS_MD};

type OnClick = Box<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct GradientButton {
    id: ElementId,
    label: SharedString,
    tokens: Tokens,
    disabled: bool,
    on_click: Option<OnClick>,
}

impl GradientButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, tokens: Tokens) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tokens,
            disabled: false,
            on_click: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

fn brighten(color: Hsla) -> Hsla {
    Hsla {
        l: (color.l + 0.05).min(1.),
        ..color
    }
}

fn glow(t: &Tokens, y: f32, blur: f32) -> BoxShadow {
    BoxShadow {
        color: t.accent_glow,
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.),
        inset: false,
    }
}

impl RenderOnce for GradientButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let gradient = |from: Hsla, to: Hsla| {
            linear_gradient(90., linear_color_stop(from, 0.), linear_color_stop(to, 1.))
        };

        div()
            .id(self.id)
            .w(px(300.))
            .max_w_full()
            .h(px(56.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(RADIUS_MD))
            .text_size(px(20.))
            .font_weight(FontWeight::BOLD)
            .text_color(t.on_accent)
            .bg(gradient(t.cta_from, t.cta_to))
            .child(self.label)
            .when(self.disabled, |this| this.opacity(0.45))
            .when(!self.disabled, |this| {
                this.cursor_pointer()
                    .shadow(vec![glow(&t, 4., 14.)])
                    .hover(|style| {
                        style
                            .bg(gradient(brighten(t.cta_from), brighten(t.cta_to)))
                            .shadow(vec![glow(&t, 6., 18.)])
                    })
            })
            .when_some(self.on_click.filter(|_| !self.disabled), |this, handler| {
                this.on_click(move |_, window, cx| handler(window, cx))
            })
    }
}
