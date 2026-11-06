use std::sync::Arc;

use gpui::{div, prelude::*, AnyView, Context, Entity, Image, Subscription, Window};
use gpui_kit::component::ActiveTheme as _;

use crate::pages::{AnimationPage, ImagePage, Page, SettingsPage, VideoPage};
use crate::settings::{self, ThemeMode};
use crate::sidebar::Sidebar;
use crate::theme::{self, ActiveTokens as _, BASE_FONT_SIZE};

pub struct Imechen {
    page: Page,
    logo: Arc<Image>,
    // Pages are entities owned here so their state survives tab switches.
    image: Entity<ImagePage>,
    video: Entity<VideoPage>,
    animation: Entity<AnimationPage>,
    settings: Entity<SettingsPage>,
    _appearance_subscription: Subscription,
}

impl Imechen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_window_appearance(window, |_, window, cx| {
            let mode = settings::theme_mode(cx);
            if mode == ThemeMode::System {
                theme::apply(mode, window.appearance(), cx);
            }
        });

        Self {
            page: Page::Image,
            logo: crate::sidebar::logo(),
            image: cx.new(|cx| ImagePage::new(window, cx)),
            video: cx.new(|cx| VideoPage::new(window, cx)),
            animation: cx.new(|cx| AnimationPage::new(window, cx)),
            settings: cx.new(|_| SettingsPage),
            _appearance_subscription: subscription,
        }
    }

    fn current_page(&self) -> AnyView {
        match self.page {
            Page::Image => self.image.clone().into(),
            Page::Video => self.video.clone().into(),
            Page::Animation => self.animation.clone().into(),
            Page::Settings => self.settings.clone().into(),
        }
    }
}

impl Render for Imechen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let this = cx.entity();

        div()
            .size_full()
            .flex()
            .bg(t.bg_base)
            .text_color(t.text_primary)
            .text_size(gpui::px(BASE_FONT_SIZE))
            .font_family(cx.theme().font_family.clone())
            .child(Sidebar::new(
                self.page,
                t,
                self.logo.clone(),
                move |page, _, cx| {
                    this.update(cx, |this, cx| {
                        this.page = page;
                        cx.notify();
                    });
                },
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(div().flex_1().min_h_0().child(self.current_page())),
            )
    }
}
