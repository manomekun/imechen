use gpui::{div, prelude::*, px, App, Context, Entity, Window};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::slider::{Slider, SliderState};
use gpui_kit::component::IndexPath;

use crate::theme::Tokens;

pub const CONTROL_HEIGHT: f32 = 40.0;
pub const CONTROL_TEXT_SIZE: f32 = 15.0;

pub type OptionSelect = SelectState<SearchableVec<&'static str>>;

pub fn select_state(
    labels: &[&'static str],
    selected: usize,
    window: &mut Window,
    cx: &mut App,
) -> Entity<OptionSelect> {
    let items = SearchableVec::new(labels.to_vec());
    cx.new(|cx: &mut Context<OptionSelect>| {
        SelectState::new(items, Some(IndexPath::new(selected)), window, cx)
    })
}

// gpui-kit paints dark-mode controls with a translucent mix of its border
// color instead of the field background, so the background is set per control.
pub fn themed_select(
    state: &Entity<OptionSelect>,
    t: Tokens,
) -> Select<SearchableVec<&'static str>> {
    Select::new(state)
        .h(px(CONTROL_HEIGHT))
        .bg(t.bg_field)
        .text_size(px(CONTROL_TEXT_SIZE))
}

pub fn slider_state(min: f32, max: f32, value: f32, cx: &mut App) -> Entity<SliderState> {
    cx.new(|_| {
        SliderState::new()
            .min(min)
            .max(max)
            .step(1.)
            .default_value(value)
    })
}

pub fn themed_slider(state: &Entity<SliderState>, disabled: bool) -> Slider {
    Slider::new(state).disabled(disabled)
}

/// Digits only, so the field can never hold text that is not a number.
pub fn dimension_input(window: &mut Window, cx: &mut App) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx).validate(|text, _| text.bytes().all(|b| b.is_ascii_digit()))
    })
}

pub fn themed_input(state: &Entity<InputState>, t: Tokens) -> Input {
    Input::new(state)
        .h(px(CONTROL_HEIGHT))
        .bg(t.bg_field)
        .text_size(px(CONTROL_TEXT_SIZE))
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidDimension;

/// Empty means "not specified". Zero and values beyond `u32` are rejected
/// rather than ignored, because resizing to nothing is never what the user
/// typed.
pub fn parse_dimension(text: &str) -> Result<Option<u32>, InvalidDimension> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    match text.parse::<u32>() {
        Ok(value) if value > 0 => Ok(Some(value)),
        _ => Err(InvalidDimension),
    }
}

#[derive(IntoElement)]
pub struct DimensionRow {
    width: Entity<InputState>,
    height: Entity<InputState>,
    tokens: Tokens,
}

impl DimensionRow {
    pub fn new(width: &Entity<InputState>, height: &Entity<InputState>, tokens: Tokens) -> Self {
        Self {
            width: width.clone(),
            height: height.clone(),
            tokens,
        }
    }
}

impl RenderOnce for DimensionRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let unit = |text: &'static str| {
            div()
                .flex_shrink_0()
                .text_size(px(CONTROL_TEXT_SIZE))
                .text_color(t.text_secondary)
                .child(text)
        };
        let input =
            |state: &Entity<InputState>| div().flex_1().min_w_0().child(themed_input(state, t));

        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(unit("幅"))
            .child(input(&self.width))
            .child(unit("×"))
            .child(unit("高さ"))
            .child(input(&self.height))
            .child(unit("px"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dimension_means_not_specified() {
        assert_eq!(parse_dimension(""), Ok(None));
        assert_eq!(parse_dimension("  "), Ok(None));
    }

    #[test]
    fn positive_integer_dimension_is_parsed() {
        assert_eq!(parse_dimension("640"), Ok(Some(640)));
        assert_eq!(parse_dimension(" 1 "), Ok(Some(1)));
    }

    #[test]
    fn zero_dimension_is_rejected_instead_of_ignored() {
        assert_eq!(parse_dimension("0"), Err(InvalidDimension));
    }

    #[test]
    fn non_numeric_and_overflowing_dimensions_are_rejected() {
        assert_eq!(parse_dimension("12a"), Err(InvalidDimension));
        assert_eq!(parse_dimension("-5"), Err(InvalidDimension));
        assert_eq!(parse_dimension("99999999999"), Err(InvalidDimension));
    }
}
