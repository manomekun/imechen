use std::sync::Arc;

use gpui::{
    div, img, prelude::*, px, App, FontWeight, Image, ImageFormat, ObjectFit, SharedString, Window,
};
use gpui_kit::component::{Icon, Sizable as _};

use crate::pages::Page;
use crate::theme::{Tokens, RADIUS_SM};

pub const WIDTH: f32 = 220.0;

const NAV: [Page; 3] = [Page::Image, Page::Video, Page::Animation];

pub fn logo() -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../assets/icon.png").to_vec(),
    ))
}

type OnSelect = Arc<dyn Fn(Page, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Sidebar {
    active: Page,
    tokens: Tokens,
    logo: Arc<Image>,
    on_select: OnSelect,
}

impl Sidebar {
    pub fn new(
        active: Page,
        tokens: Tokens,
        logo: Arc<Image>,
        on_select: impl Fn(Page, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            active,
            tokens,
            logo,
            on_select: Arc::new(on_select),
        }
    }

    fn item(&self, page: Page) -> impl IntoElement {
        let t = self.tokens;
        let active = page == self.active;
        let on_select = self.on_select.clone();
        let id: SharedString = format!("nav-{page:?}").into();

        div()
            .id(id)
            .h(px(44.))
            .px(px(14.))
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(14.))
            .rounded(px(RADIUS_SM))
            .cursor_pointer()
            .text_color(t.text_primary)
            .when(active, |this| this.bg(t.accent_tint))
            .when(!active, |this| this.hover(|style| style.bg(t.bg_hover)))
            .on_click(move |_, window, cx| on_select(page, window, cx))
            .child(
                Icon::new(page.icon())
                    .with_size(px(22.))
                    .text_color(if active { t.accent } else { t.text_secondary }),
            )
            .child(
                div()
                    .text_size(px(15.))
                    .whitespace_nowrap()
                    .when(active, |this| this.font_weight(FontWeight::MEDIUM))
                    .child(page.label()),
            )
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;

        div()
            .w(px(WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .pt(px(28.))
            .px(px(14.))
            .pb(px(16.))
            .bg(t.bg_sidebar)
            .border_r_1()
            .border_color(t.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .pt_0()
                    .px(px(8.))
                    .pb(px(24.))
                    .child(
                        img(self.logo.clone())
                            .size(px(26.))
                            .flex_shrink_0()
                            .rounded(px(RADIUS_SM))
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(t.text_primary)
                            .child("imechen"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .children(NAV.into_iter().map(|page| self.item(page))),
            )
            .child(
                div()
                    .pt(px(12.))
                    .border_t_1()
                    .border_color(t.border_strong)
                    .child(self.item(Page::Settings)),
            )
    }
}
