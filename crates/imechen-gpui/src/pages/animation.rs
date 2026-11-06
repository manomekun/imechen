use gpui::{prelude::*, Context, Window};

use super::{title_only, Page};
use crate::theme::ActiveTokens as _;

pub struct AnimationPage;

impl Render for AnimationPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        title_only(Page::Animation, &cx.tokens())
    }
}
