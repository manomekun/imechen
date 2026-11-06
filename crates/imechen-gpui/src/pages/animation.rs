use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    div, prelude::*, px, AnyElement, Context, Entity, Image, ImageFormat, PathPromptOptions,
    ScrollHandle, SharedString, Subscription, Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::SelectEvent;
use gpui_kit::component::{Icon, Sizable as _};
use imechen_core::animation::{create_animation, CreateAnimationRequest};
use imechen_core::image as core_image;

use super::Page;
use crate::components::{
    batch_status, dimension_input, select_state, text_cell, themed_input, themed_select, tooltip,
    BatchState, Column, ConvertError, Field, FileRow, FileTable, Footer, OptionSelect, OutputItem,
    PageShell, PathPicker, PixelLoader, ResultList, Selection, Thumb, Toolbar,
};
use crate::media::format_size;
use crate::output::{default_output_dir, file_name};
use crate::theme::{ActiveTokens as _, Tokens, RADIUS_SM};

const DEFAULT_NAME: &str = "animation";
const DEFAULT_FPS: u32 = 24;
const FPS_RANGE: std::ops::RangeInclusive<u32> = 1..=60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnimFormat {
    Gif,
    Awebp,
    Mp4,
}

impl AnimFormat {
    const ALL: [Self; 3] = [Self::Gif, Self::Awebp, Self::Mp4];

    fn label(self) -> &'static str {
        match self {
            Self::Gif => "GIF",
            Self::Awebp => "Animated WebP",
            Self::Mp4 => "MP4",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.label() == label)
    }

    fn request_name(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Awebp => "awebp",
            Self::Mp4 => "mp4",
        }
    }

    /// Must match the extension core gives its output file.
    fn extension(self) -> &'static str {
        match self {
            Self::Awebp => "webp",
            other => other.request_name(),
        }
    }
}

/// Splits into alternating non-digit and digit runs.
fn natural_chunks(s: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut prev_digit = None;
    for (i, c) in s.char_indices() {
        let digit = c.is_ascii_digit();
        if prev_digit.is_some_and(|p| p != digit) {
            chunks.push(&s[start..i]);
            start = i;
        }
        prev_digit = Some(digit);
    }
    if start < s.len() {
        chunks.push(&s[start..]);
    }
    chunks
}

/// Numeric-aware ordering (`frame2` < `frame10`). Digit runs are compared as
/// strings of digits rather than integers so arbitrarily long runs cannot overflow.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a_chunks, b_chunks) = (natural_chunks(a), natural_chunks(b));
    for (x, y) in a_chunks.iter().zip(&b_chunks) {
        let numeric = x.starts_with(|c: char| c.is_ascii_digit())
            && y.starts_with(|c: char| c.is_ascii_digit());
        let order = if numeric {
            let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
            x.len().cmp(&y.len()).then_with(|| x.cmp(y))
        } else {
            x.to_lowercase().cmp(&y.to_lowercase())
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a_chunks.len().cmp(&b_chunks.len()).then_with(|| a.cmp(b))
}

fn sort_naturally(paths: &mut [PathBuf]) {
    paths.sort_by(|a, b| natural_cmp(&file_name(a), &file_name(b)));
}

/// Out-of-range indices and `from == to` leave the list untouched.
fn move_file<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from == to || from >= items.len() || to >= items.len() {
        return;
    }
    let item = items.remove(from);
    items.insert(to, item);
}

/// A UI pre-check of core's rule so the mistake is reported before anything
/// starts; core still validates on its own.
fn validate_output_name(name: &str) -> Result<&str, &'static str> {
    let is_bare = !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\']);
    if is_bare {
        Ok(name)
    } else {
        Err("ファイル名が正しくありません")
    }
}

fn parse_fps(text: &str) -> Result<u32, &'static str> {
    match text.trim().parse::<u32>() {
        Ok(fps) if FPS_RANGE.contains(&fps) => Ok(fps),
        _ => Err("FPS は 1〜60 の整数で指定してください"),
    }
}

fn can_create(file_count: usize, creating: bool) -> bool {
    file_count > 0 && !creating
}

struct Frame {
    path: PathBuf,
    thumbnail: Arc<Image>,
    name: SharedString,
    dimensions: SharedString,
    size: SharedString,
}

struct Created {
    name: SharedString,
    detail: SharedString,
    path: PathBuf,
}

enum Loaded {
    Frame(Frame),
    Failed(ConvertError),
}

pub struct AnimationPage {
    files: Vec<Frame>,
    selection: Selection,
    format: AnimFormat,
    format_select: Entity<OptionSelect>,
    name: Entity<InputState>,
    fps: Entity<InputState>,
    output_dir: PathBuf,
    creating: bool,
    result: Option<Created>,
    errors: Vec<ConvertError>,
    notice: Option<SharedString>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl AnimationPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let labels = AnimFormat::ALL.map(AnimFormat::label);
        let format_select = select_state(&labels, 0, window, cx);
        let subscriptions = vec![cx.subscribe(
            &format_select,
            |this, _, event: &SelectEvent<SearchableVec<&'static str>>, cx| {
                let SelectEvent::Confirm(label) = event;
                if let Some(format) = label.and_then(AnimFormat::from_label) {
                    this.format = format;
                    cx.notify();
                }
            },
        )];

        let name = cx.new(|cx| InputState::new(window, cx));
        name.update(cx, |input, cx| input.set_value(DEFAULT_NAME, window, cx));
        let fps = dimension_input(window, cx);
        fps.update(cx, |input, cx| {
            input.set_value(DEFAULT_FPS.to_string(), window, cx)
        });

        Self {
            files: Vec::new(),
            selection: Selection::default(),
            format: AnimFormat::Gif,
            format_select,
            name,
            fps,
            output_dir: default_output_dir(),
            creating: false,
            result: None,
            errors: Vec::new(),
            notice: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Each batch is ordered on its own and appended, so frames added later
    /// never reshuffle the ones already arranged by hand.
    pub fn add_paths(&mut self, mut paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        sort_naturally(&mut paths);
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(
                            |path| match core_image::get_image_info(&path.to_string_lossy()) {
                                Ok(info) => Loaded::Frame(Frame {
                                    thumbnail: Arc::new(Image::from_bytes(
                                        ImageFormat::Png,
                                        info.thumbnail_png,
                                    )),
                                    name: info.name.into(),
                                    dimensions: format!("{}×{}", info.width, info.height).into(),
                                    size: format_size(info.size).into(),
                                    path,
                                }),
                                Err(message) => Loaded::Failed(ConvertError {
                                    name: file_name(&path),
                                    path,
                                    message: message.into(),
                                }),
                            },
                        )
                        .collect::<Vec<_>>()
                })
                .await;
            this.update(cx, |this, cx| {
                for item in loaded {
                    match item {
                        Loaded::Frame(frame) => this.files.push(frame),
                        Loaded::Failed(error) => this.errors.push(error),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn pick_files(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                this.update(cx, |this, cx| this.add_paths(paths, cx)).ok();
            }
        })
        .detach();
    }

    fn pick_output_dir(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = paths.await {
                if let Some(dir) = paths.pop() {
                    this.update(cx, |this, cx| {
                        this.output_dir = dir;
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn remove_selected(&mut self, cx: &mut Context<Self>) {
        let selection = &self.selection;
        self.files.retain(|file| !selection.contains(&file.path));
        self.selection
            .retain_present(self.files.iter().map(|f| f.path.as_path()));
        cx.notify();
    }

    fn toggle_all(&mut self, cx: &mut Context<Self>) {
        self.selection
            .toggle_all(self.files.iter().map(|f| f.path.as_path()));
        cx.notify();
    }

    fn move_frame(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        move_file(&mut self.files, from, to);
        cx.notify();
    }

    fn reject(&mut self, message: &str, cx: &mut Context<Self>) {
        self.notice = Some(message.to_string().into());
        cx.notify();
    }

    fn create(&mut self, cx: &mut Context<Self>) {
        if !can_create(self.files.len(), self.creating) {
            return;
        }
        let name = self.name.read(cx).value().to_string();
        let name = match validate_output_name(&name) {
            Ok(name) => name.to_string(),
            Err(message) => return self.reject(message, cx),
        };
        let fps = match parse_fps(&self.fps.read(cx).value()) {
            Ok(fps) => fps,
            Err(message) => return self.reject(message, cx),
        };
        let format = self.format;
        let output_dir = self.output_dir.clone();
        let target = output_dir.join(format!("{name}.{}", format.extension()));

        self.notice = None;
        self.creating = true;
        self.result = None;
        self.errors.clear();
        cx.notify();

        let request = CreateAnimationRequest {
            image_paths: self
                .files
                .iter()
                .map(|f| f.path.to_string_lossy().into_owned())
                .collect(),
            output_format: format.request_name().to_string(),
            output_dir: output_dir.to_string_lossy().into_owned(),
            output_name: name,
            fps,
        };

        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { create_animation(request) })
                .await;
            this.update(cx, |this, cx| {
                this.creating = false;
                match outcome {
                    Ok(result) => {
                        let output = PathBuf::from(&result.output_path);
                        this.result = Some(Created {
                            name: file_name(&output),
                            detail: format!(
                                "{} フレーム · {}",
                                result.frame_count,
                                format_size(result.output_size)
                            )
                            .into(),
                            path: output,
                        });
                        this.files.clear();
                        this.selection.retain_present(std::iter::empty());
                    }
                    Err(message) => this.errors.push(ConvertError {
                        name: file_name(&target),
                        path: target,
                        message: message.into(),
                    }),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn rows(&self, t: Tokens, this: &Entity<Self>) -> Vec<FileRow> {
        let last = self.files.len().saturating_sub(1);
        self.files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let reorder = div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(2.))
                    .child(reorder_button(
                        this.clone(),
                        ("move-up", i),
                        IconName::ChevronUp,
                        "上へ移動",
                        (i > 0).then(|| i - 1),
                        i,
                        t,
                    ))
                    .child(reorder_button(
                        this.clone(),
                        ("move-down", i),
                        IconName::ChevronDown,
                        "下へ移動",
                        (i < last).then_some(i + 1),
                        i,
                        t,
                    ))
                    .into_any_element();
                FileRow {
                    path: file.path.clone(),
                    name: file.name.clone(),
                    thumb: Thumb::Image(file.thumbnail.clone()),
                    selected: self.selection.contains(&file.path),
                    cells: vec![
                        index_cell(i + 1, t),
                        text_cell(file.dimensions.clone()),
                        text_cell(file.size.clone()),
                        reorder,
                    ],
                }
            })
            .collect()
    }

    fn notice(&self, t: &Tokens) -> Option<impl IntoElement> {
        let message = self.notice.clone()?;
        Some(
            div()
                .self_start()
                .py(px(8.))
                .px(px(14.))
                .rounded(px(RADIUS_SM))
                .bg(t.error_tint)
                .text_size(px(13.))
                .text_color(t.error)
                .child(message),
        )
    }
}

fn reorder_button(
    this: Entity<AnimationPage>,
    id: (&'static str, usize),
    icon: IconName,
    label: &'static str,
    target: Option<usize>,
    index: usize,
    t: Tokens,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .p(px(2.))
        .rounded(px(4.))
        .text_color(t.text_secondary)
        .child(Icon::new(icon).with_size(px(14.)))
        .tooltip(tooltip(label))
        .map(|button| match target {
            Some(to) => button
                .cursor_pointer()
                .hover(|style| style.bg(t.bg_hover).text_color(t.accent))
                .on_click(move |_, _, cx| {
                    this.update(cx, |this, cx| this.move_frame(index, to, cx))
                }),
            None => button.opacity(0.25),
        })
}

fn index_cell(number: usize, t: Tokens) -> AnyElement {
    div()
        .w_full()
        .text_right()
        .text_size(px(12.))
        .text_color(t.text_muted)
        .child(number.to_string())
        .into_any_element()
}

impl Render for AnimationPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let this = cx.entity();

        let status = batch_status(BatchState {
            file_count: self.files.len(),
            ..BatchState::default()
        });

        let header_state = self
            .selection
            .state_over(self.files.iter().map(|f| f.path.as_path()));
        let columns = vec![
            Column::cell("#", 24.),
            Column::thumb(IconName::Image),
            Column::name(),
            Column::cell("画像サイズ", 92.),
            Column::cell("ファイルサイズ", 96.),
            Column::cell("", 28.),
        ];
        let table = FileTable::new(self.rows(t, &this), columns, header_state, t)
            .empty(
                IconName::Images,
                "連番画像をドロップまたはクリックして選択",
                "PNG, JPEG, BMP（複数選択可）",
            )
            .scroll_offset(self.scroll.offset().y)
            .on_toggle({
                let this = this.clone();
                move |path, _, cx| {
                    let path = path.to_path_buf();
                    this.update(cx, |this, cx| {
                        this.selection.toggle(&path);
                        cx.notify();
                    });
                }
            })
            .on_toggle_all({
                let this = this.clone();
                move |_, cx| this.update(cx, |this, cx| this.toggle_all(cx))
            })
            .on_pick({
                let this = this.clone();
                move |_, cx| this.update(cx, |this, cx| this.pick_files(cx))
            })
            .on_drop({
                let this = this.clone();
                move |paths, _, cx| this.update(cx, |this, cx| this.add_paths(paths, cx))
            });

        let outputs = self
            .result
            .iter()
            .map(|r| OutputItem {
                name: r.name.clone(),
                detail: r.detail.clone(),
                path: r.path.clone(),
                full_path: Some(r.path.display().to_string().into()),
            })
            .collect();
        let errors = self
            .errors
            .iter()
            .map(|e| ConvertError {
                name: e.name.clone(),
                path: e.path.clone(),
                message: e.message.clone(),
            })
            .collect();

        let fps_field = Field::new("FPS", t).child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .w(px(96.))
                        .flex_shrink_0()
                        .child(themed_input(&self.fps, t)),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(t.text_secondary)
                        .child("フレーム/秒"),
                ),
        );

        div()
            .size_full()
            .child(
                PageShell::new(Page::Animation.title(), "出力設定", &self.scroll, t)
                    .notice(self.notice(&t))
                    .toolbar(
                        Toolbar::new(t)
                            .delete_disabled(self.selection.count() == 0)
                            .on_add({
                                let this = this.clone();
                                move |_, cx| this.update(cx, |this, cx| this.pick_files(cx))
                            })
                            .on_delete({
                                let this = this.clone();
                                move |_, cx| this.update(cx, |this, cx| this.remove_selected(cx))
                            }),
                    )
                    .field(Field::new("出力形式", t).child(themed_select(&self.format_select, t)))
                    .field(Field::new("ファイル名", t).child(themed_input(&self.name, t)))
                    .field(fps_field)
                    .field(Field::new("保存先", t).child({
                        let this = this.clone();
                        PathPicker::new(&self.output_dir, t).on_pick(move |_, cx| {
                            this.update(cx, |this, cx| this.pick_output_dir(cx))
                        })
                    }))
                    .footer(
                        Footer::new(status, 0, "生成する", t)
                            .cta_disabled(!can_create(self.files.len(), self.creating))
                            .on_cta({
                                let this = this.clone();
                                move |_, cx| this.update(cx, |this, cx| this.create(cx))
                            }),
                    )
                    .child(table)
                    .child(ResultList::new(outputs, errors, t)),
            )
            .when(self.creating, |this| {
                this.child(PixelLoader::new("アニメーション生成中..."))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(paths: &[&str]) -> Vec<String> {
        let mut paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        sort_naturally(&mut paths);
        paths.iter().map(|p| file_name(p).to_string()).collect()
    }

    #[test]
    fn numbers_in_names_sort_by_value_not_by_digit() {
        assert_eq!(
            names(&["/a/frame10.png", "/a/frame2.png", "/a/frame1.png"]),
            ["frame1.png", "frame2.png", "frame10.png"]
        );
    }

    #[test]
    fn natural_order_compares_file_names_not_directories() {
        assert_eq!(names(&["/z/a1.png", "/a/b2.png"]), ["a1.png", "b2.png"]);
    }

    #[test]
    fn natural_order_ignores_case() {
        assert_eq!(names(&["B.png", "a.png"]), ["a.png", "B.png"]);
    }

    #[test]
    fn natural_order_handles_digit_runs_longer_than_any_integer() {
        let big = "f99999999999999999999999.png";
        let bigger = "f100000000000000000000000.png";
        assert_eq!(names(&[bigger, big]), [big, bigger]);
    }

    #[test]
    fn a_prefix_sorts_before_its_extension() {
        assert_eq!(names(&["a1b.png", "a1.png"]), ["a1.png", "a1b.png"]);
    }

    #[test]
    fn moving_a_file_shifts_the_ones_in_between() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        move_file(&mut v, 0, 2);
        assert_eq!(v, ['b', 'c', 'a', 'd']);
        move_file(&mut v, 3, 1);
        assert_eq!(v, ['b', 'd', 'c', 'a']);
    }

    #[test]
    fn moving_to_the_same_slot_or_out_of_range_changes_nothing() {
        let mut v = vec![1, 2, 3];
        move_file(&mut v, 1, 1);
        move_file(&mut v, 0, 3);
        move_file(&mut v, 5, 0);
        assert_eq!(v, [1, 2, 3]);
    }

    #[test]
    fn adjacent_swaps_at_the_ends_work() {
        let mut v = vec![1, 2, 3];
        move_file(&mut v, 0, 1);
        move_file(&mut v, 2, 1);
        assert_eq!(v, [2, 3, 1]);
    }

    #[test]
    fn bare_output_names_are_accepted() {
        assert_eq!(validate_output_name("animation"), Ok("animation"));
        assert_eq!(validate_output_name("my anim.v2"), Ok("my anim.v2"));
    }

    #[test]
    fn output_names_that_are_empty_or_escape_the_directory_are_rejected() {
        for name in ["", ".", "..", "a/b", "../x", "a\\b", "/abs"] {
            assert!(validate_output_name(name).is_err(), "{name:?}");
        }
    }

    #[test]
    fn fps_within_one_to_sixty_is_accepted() {
        assert_eq!(parse_fps("1"), Ok(1));
        assert_eq!(parse_fps(" 24 "), Ok(24));
        assert_eq!(parse_fps("60"), Ok(60));
    }

    #[test]
    fn fps_outside_the_range_or_not_a_number_is_rejected() {
        for text in ["", "0", "61", "-1", "abc", "2.5", "99999999999"] {
            assert!(parse_fps(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn creating_needs_files_and_no_run_in_flight() {
        assert!(can_create(2, false));
        assert!(!can_create(0, false));
        assert!(!can_create(2, true));
    }

    #[test]
    fn format_labels_and_extensions_match_the_original_options() {
        for format in AnimFormat::ALL {
            assert_eq!(AnimFormat::from_label(format.label()), Some(format));
        }
        assert_eq!(AnimFormat::Awebp.request_name(), "awebp");
        assert_eq!(AnimFormat::Awebp.extension(), "webp");
        assert_eq!(AnimFormat::from_label("APNG"), None);
    }
}
