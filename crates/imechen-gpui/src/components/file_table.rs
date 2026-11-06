use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{
    div, img, prelude::*, px, AnyElement, App, ExternalPaths, ObjectFit, Pixels, SharedString,
    Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};

use super::{tooltip, CheckState, Checkbox};
use crate::theme::{Tokens, RADIUS_MD, RADIUS_SM};

const HEADER_HEIGHT: f32 = 38.0;
const THUMB_WIDTH: f32 = 110.0;
// Rows have a fixed height (thumbnail box plus padding and divider), which the
// sticky header needs to know where the table ends.
const THUMB_HEIGHT: f32 = 66.0;
const ROW_HEIGHT: f32 = THUMB_HEIGHT + 24.0 + 1.0;
const CHECKBOX_COLUMN: f32 = 40.0;
const NAME_MIN_WIDTH: f32 = 96.0;
const COLUMN_GAP: f32 = 10.0;

pub enum ColumnKind {
    Thumb {
        fallback: IconName,
    },
    Name,
    /// Consumes the next entry of each row's `cells`.
    Cell {
        width: f32,
        nowrap: bool,
    },
}

pub struct Column {
    pub header: SharedString,
    pub kind: ColumnKind,
}

impl Column {
    pub fn thumb(fallback: IconName) -> Self {
        Self {
            header: SharedString::default(),
            kind: ColumnKind::Thumb { fallback },
        }
    }

    pub fn name() -> Self {
        Self {
            header: "ファイル名".into(),
            kind: ColumnKind::Name,
        }
    }

    pub fn cell(header: impl Into<SharedString>, width: f32) -> Self {
        Self {
            header: header.into(),
            kind: ColumnKind::Cell {
                width,
                nowrap: true,
            },
        }
    }
}

pub enum Thumb {
    Path(PathBuf),
    None,
}

/// A plain text cell for [`FileRow::cells`].
pub fn text_cell(text: impl Into<SharedString>) -> AnyElement {
    div().child(text.into()).into_any_element()
}

pub struct FileRow {
    pub path: PathBuf,
    pub name: SharedString,
    pub thumb: Thumb,
    pub selected: bool,
    /// One element per [`ColumnKind::Cell`] column, in column order.
    pub cells: Vec<AnyElement>,
}

struct EmptyState {
    icon: IconName,
    label: SharedString,
    hint: SharedString,
}

type OnPath = Rc<dyn Fn(&Path, &mut Window, &mut App) + 'static>;
type OnAction = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type OnDrop = Rc<dyn Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct FileTable {
    rows: Vec<FileRow>,
    columns: Vec<Column>,
    header_state: CheckState,
    empty: EmptyState,
    // Negative while the enclosing scroll area is scrolled down.
    scroll_offset: Pixels,
    tokens: Tokens,
    on_toggle: Option<OnPath>,
    on_toggle_all: Option<OnAction>,
    on_pick: Option<OnAction>,
    on_drop: Option<OnDrop>,
}

impl FileTable {
    pub fn new(
        rows: Vec<FileRow>,
        columns: Vec<Column>,
        header_state: CheckState,
        tokens: Tokens,
    ) -> Self {
        Self {
            rows,
            columns,
            header_state,
            empty: EmptyState {
                icon: IconName::Image,
                label: SharedString::default(),
                hint: SharedString::default(),
            },
            scroll_offset: px(0.),
            tokens,
            on_toggle: None,
            on_toggle_all: None,
            on_pick: None,
            on_drop: None,
        }
    }

    pub fn empty(
        mut self,
        icon: IconName,
        label: impl Into<SharedString>,
        hint: impl Into<SharedString>,
    ) -> Self {
        self.empty = EmptyState {
            icon,
            label: label.into(),
            hint: hint.into(),
        };
        self
    }

    pub fn scroll_offset(mut self, offset: Pixels) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn on_toggle(mut self, handler: impl Fn(&Path, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub fn on_toggle_all(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_toggle_all = Some(Rc::new(handler));
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    pub fn on_drop(
        mut self,
        handler: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drop = Some(Rc::new(handler));
        self
    }
}

fn thumb_box(row_thumb: Thumb, fallback: IconName, t: &Tokens) -> impl IntoElement {
    let t = *t;
    let fallback_icon = move || {
        Icon::new(fallback)
            .with_size(px(22.))
            .text_color(t.text_muted)
            .into_any_element()
    };
    let content = match row_thumb {
        Thumb::Path(path) => img(path)
            .size_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(fallback_icon)
            .into_any_element(),
        Thumb::None => fallback_icon(),
    };
    div()
        .w_full()
        .h(px(THUMB_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .rounded(px(RADIUS_SM))
        .bg(t.thumb_bg)
        .child(content)
}

fn fixed_cell(width: f32) -> gpui::Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .min_w_0()
        .overflow_hidden()
}

impl FileTable {
    fn header(&self, t: &Tokens) -> gpui::Div {
        let checkbox = {
            let mut checkbox = Checkbox::new("select-all", self.header_state, *t);
            if let Some(handler) = self.on_toggle_all.clone() {
                checkbox = checkbox.on_toggle(move |window, cx| handler(window, cx));
            }
            checkbox
        };

        div()
            .flex()
            .items_center()
            .gap(px(COLUMN_GAP))
            .px(px(16.))
            .h(px(HEADER_HEIGHT))
            .bg(t.bg_header)
            .border_b_1()
            .border_color(t.border)
            .text_size(px(13.))
            .text_color(t.text_secondary)
            .child(fixed_cell(CHECKBOX_COLUMN).child(checkbox))
            .children(self.columns.iter().map(|column| {
                match column.kind {
                    ColumnKind::Thumb { .. } => div()
                        .w(px(THUMB_WIDTH))
                        .flex_shrink(1.)
                        .min_w_0()
                        .child(column.header.clone()),
                    ColumnKind::Name => div()
                        .flex_1()
                        .min_w(px(NAME_MIN_WIDTH))
                        .truncate()
                        .child(column.header.clone()),
                    ColumnKind::Cell { width, .. } => fixed_cell(width)
                        .whitespace_nowrap()
                        .truncate()
                        .child(column.header.clone()),
                }
            }))
    }

    fn row(&self, index: usize, row: FileRow, t: &Tokens) -> gpui::Stateful<gpui::Div> {
        let FileRow {
            path,
            name,
            thumb,
            selected,
            cells,
        } = row;

        let checkbox = {
            let state = if selected {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            };
            let mut checkbox = Checkbox::new(("select-row", index), state, *t);
            if let Some(handler) = self.on_toggle.clone() {
                let path = path.clone();
                checkbox = checkbox.on_toggle(move |window, cx| handler(&path, window, cx));
            }
            checkbox
        };

        let mut thumb = Some(thumb);
        let mut cells = cells.into_iter();
        let mut row = div()
            .id(("file-row", index))
            .flex()
            .items_center()
            .gap(px(COLUMN_GAP))
            .px(px(16.))
            .h(px(ROW_HEIGHT))
            .border_b_1()
            .border_color(t.border)
            .text_size(px(13.))
            .child(fixed_cell(CHECKBOX_COLUMN).child(checkbox));

        for column in &self.columns {
            row = match column.kind {
                ColumnKind::Thumb { fallback } => row.child(
                    div()
                        .w(px(THUMB_WIDTH))
                        .flex_shrink(1.)
                        .min_w_0()
                        .child(thumb_box(thumb.take().unwrap_or(Thumb::None), fallback, t)),
                ),
                ColumnKind::Name => row.child(
                    div()
                        .id(("file-name", index))
                        .flex_1()
                        .min_w(px(NAME_MIN_WIDTH))
                        .truncate()
                        .text_size(px(15.))
                        .text_color(t.text_primary)
                        .tooltip(tooltip(path.display().to_string()))
                        .child(name.clone()),
                ),
                ColumnKind::Cell { width, nowrap } => row.child(
                    fixed_cell(width)
                        .text_color(t.text_primary)
                        .when(nowrap, |this| this.whitespace_nowrap())
                        .children(cells.next()),
                ),
            };
        }
        row
    }

    fn empty_state(&self, t: &Tokens) -> gpui::Stateful<gpui::Div> {
        let t = *t;
        let on_pick = self.on_pick.clone();
        div()
            .id("file-table-empty")
            .w_full()
            .min_h(px(240.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.))
            .border_2()
            .border_dashed()
            .border_color(t.border_strong)
            .rounded(px(RADIUS_MD))
            .text_color(t.text_muted)
            .cursor_pointer()
            .hover(|style| style.border_color(t.accent).text_color(t.accent))
            .child(Icon::new(self.empty.icon).with_size(px(40.)))
            .child(
                div()
                    .text_size(px(15.))
                    .text_color(t.text_secondary)
                    .child(self.empty.label.clone()),
            )
            .child(div().text_size(px(12.)).child(self.empty.hint.clone()))
            .when_some(on_pick, |this, handler| {
                this.on_click(move |_, window, cx| handler(window, cx))
            })
    }
}

impl RenderOnce for FileTable {
    fn render(mut self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        let populated = !self.rows.is_empty();
        let on_drop = self.on_drop.clone();

        let body = if populated {
            // The header is drawn last at an offset so it stays visible while
            // the rows scroll under it; gpui has no sticky positioning.
            let table_end = self.rows.len() as f32 * ROW_HEIGHT;
            let pinned = (-f32::from(self.scroll_offset)).clamp(0., table_end);
            let header = self.header(&t);
            let rows = std::mem::take(&mut self.rows);
            let rows: Vec<_> = rows
                .into_iter()
                .enumerate()
                .map(|(i, row)| self.row(i, row, &t))
                .collect();
            div()
                .relative()
                .child(div().h(px(HEADER_HEIGHT)))
                .children(rows)
                .child(
                    header
                        .absolute()
                        .top(px(pinned))
                        .left_0()
                        .right_0()
                        .occlude(),
                )
                .into_any_element()
        } else {
            self.empty_state(&t).into_any_element()
        };

        div()
            .id("file-table")
            .min_h(px(240.))
            .overflow_hidden()
            .rounded(px(RADIUS_MD))
            .border_1()
            .border_color(gpui::transparent_black())
            .drag_over::<ExternalPaths>(move |style, _, _, _| {
                style.border_color(t.accent).bg(t.accent_tint)
            })
            .when_some(on_drop, |this, handler| {
                this.on_drop(move |paths: &ExternalPaths, window, cx| {
                    handler(paths.paths().to_vec(), window, cx)
                })
            })
            .child(body)
    }
}
