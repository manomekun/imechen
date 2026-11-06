use gpui::{prelude::*, Context, Window};

use super::{title_only, Page};
use crate::theme::ActiveTokens as _;

pub struct ImagePage;

impl Render for ImagePage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        title_only(Page::Image, &cx.tokens())
    }
}
