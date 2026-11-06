use gpui::{prelude::*, Context, Window};

use super::{title_only, Page};
use crate::theme::ActiveTokens as _;

pub struct VideoPage;

impl Render for VideoPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        title_only(Page::Video, &cx.tokens())
    }
}
