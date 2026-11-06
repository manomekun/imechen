mod checkbox;
mod controls;
mod field;
mod file_table;
mod footer;
mod gradient_button;
mod page_shell;
mod path_picker;
mod pixel_loader;
mod progress_bar;
mod result_list;
mod selection;
mod toolbar;

pub use checkbox::Checkbox;
pub use controls::{
    dimension_input, parse_resize, select_state, slider_state, themed_select, themed_slider,
    DimensionRow, OptionSelect,
};
pub use field::{Field, NoteText, ValueBadge};
pub use file_table::{text_cell, Column, FileRow, FileTable, Thumb};
pub use footer::{batch_status, BatchState, Footer};
pub use gradient_button::GradientButton;
pub use page_shell::PageShell;
pub use path_picker::PathPicker;
pub use pixel_loader::PixelLoader;
pub use progress_bar::ProgressBar;
pub use result_list::{ConvertError, OutputItem, ResultList};
pub use selection::{CheckState, Selection};
pub use toolbar::Toolbar;

use gpui::{AnyView, App, SharedString, Window};
use gpui_kit::component::tooltip::Tooltip;

pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView {
    let text = text.into();
    move |window, cx| Tooltip::new(text.clone()).build(window, cx)
}
