use std::path::{Path, PathBuf};

use futures::{channel::mpsc, StreamExt as _};
use gpui::{
    div, prelude::*, px, AnyElement, Context, Entity, PathPromptOptions, ScrollHandle,
    SharedString, Subscription, Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::SelectEvent;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use imechen_core::video::{self as core_video, ConvertVideoRequest};

use super::Page;
use crate::components::{
    batch_status, dimension_input, parse_resize, select_state, slider_state, text_cell,
    themed_select, themed_slider, BatchState, Column, ConvertError, DimensionRow, Field, FileRow,
    FileTable, Footer, OptionSelect, OutputItem, PageShell, PathPicker, PixelLoader, ResultList,
    Selection, Thumb, Toolbar, ValueBadge,
};
use crate::media::{format_duration, format_size};
use crate::output::{default_output_dir, file_name, is_same_file, target_path};
use crate::theme::{ActiveTokens as _, Tokens, RADIUS_SM};

const SAME_PATH_MESSAGE: &str = "出力先が入力ファイルと同じです";
const FFMPEG_UNAVAILABLE_MESSAGE: &str = "ffmpeg を利用できません";
const DEFAULT_QUALITY: f32 = 75.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VideoFormat {
    Mp4,
    Mov,
    Avi,
    Webm,
    Flv,
}

impl VideoFormat {
    const ALL: [Self; 5] = [Self::Mp4, Self::Mov, Self::Avi, Self::Webm, Self::Flv];

    fn label(self) -> &'static str {
        match self {
            Self::Mp4 => "MP4",
            Self::Mov => "MOV",
            Self::Avi => "AVI",
            Self::Webm => "WebM",
            Self::Flv => "FLV",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.label() == label)
    }

    /// Core takes the extension itself as the format name and falls back to
    /// MP4 for anything else, so the two must stay identical.
    fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mov => "mov",
            Self::Avi => "avi",
            Self::Webm => "webm",
            Self::Flv => "flv",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preset {
    Original,
    Height(u32),
    Custom,
}

impl Preset {
    const ALL: [(&'static str, Self); 5] = [
        ("オリジナル", Self::Original),
        ("1080p", Self::Height(1080)),
        ("720p", Self::Height(720)),
        ("480p", Self::Height(480)),
        ("カスタム", Self::Custom),
    ];

    fn labels() -> [&'static str; 5] {
        Self::ALL.map(|(label, _)| label)
    }

    fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|(l, _)| *l == label)
            .map(|(_, preset)| preset)
    }

    fn shows_dimensions(self) -> bool {
        self != Self::Original
    }

    fn dimensions_editable(self) -> bool {
        self == Self::Custom
    }
}

/// Width that keeps the source aspect ratio at `height`.
fn scaled_width(height: u32, (width, source_height): (u32, u32)) -> Option<u32> {
    if source_height == 0 {
        return None;
    }
    Some((f64::from(height) * f64::from(width) / f64::from(source_height)).round() as u32)
}

/// What the dimension fields show for `preset` given the first file, which is
/// the one the presets are derived from.
fn preset_dimensions(preset: Preset, source: (u32, u32)) -> (u32, u32) {
    match preset {
        Preset::Height(height) => (scaled_width(height, source).unwrap_or(source.0), height),
        Preset::Original | Preset::Custom => source,
    }
}

/// Values for the fields right after the user picks a preset. Custom keeps
/// whatever is typed.
fn preset_change_fill(preset: Preset, first: Option<(u32, u32)>) -> Option<(u32, u32)> {
    match preset {
        Preset::Custom => None,
        _ => first.map(|source| preset_dimensions(preset, source)),
    }
}

fn resize_for(
    preset: Preset,
    custom: (Option<u32>, Option<u32>),
    first: Option<(u32, u32)>,
) -> (Option<u32>, Option<u32>) {
    match preset {
        Preset::Original => (None, None),
        Preset::Custom => custom,
        Preset::Height(height) => (first.and_then(|f| scaled_width(height, f)), Some(height)),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ffmpeg {
    Preparing,
    Ready,
    Failed(SharedString),
}

fn cta_disabled(file_count: usize, converting: bool, ffmpeg: &Ffmpeg) -> bool {
    file_count == 0 || converting || *ffmpeg != Ffmpeg::Ready
}

fn percent_of(percent: f64) -> u32 {
    percent.round().clamp(0.0, 100.0) as u32
}

fn precheck(source: &Path, dir: &Path, format: VideoFormat) -> Result<(), &'static str> {
    if is_same_file(source, &target_path(source, dir, format.extension())) {
        Err(SAME_PATH_MESSAGE)
    } else {
        Ok(())
    }
}

struct VideoFile {
    path: PathBuf,
    name: SharedString,
    width: u32,
    height: u32,
    dimensions: SharedString,
    duration: SharedString,
    size: SharedString,
}

struct Converted {
    path: PathBuf,
    name: SharedString,
    detail: SharedString,
}

enum Loaded {
    File(VideoFile),
    Failed(ConvertError),
}

pub struct VideoPage {
    files: Vec<VideoFile>,
    selection: Selection,
    format: VideoFormat,
    format_select: Entity<OptionSelect>,
    preset: Preset,
    preset_select: Entity<OptionSelect>,
    width: Entity<InputState>,
    height: Entity<InputState>,
    quality: Entity<SliderState>,
    output_dir: PathBuf,
    ffmpeg: Ffmpeg,
    /// Probing needs ffmpeg, so paths added while it is being prepared wait here.
    pending: Vec<PathBuf>,
    converting: bool,
    progress: u32,
    results: Vec<Converted>,
    errors: Vec<ConvertError>,
    notice: Option<SharedString>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl VideoPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let format_select = select_state(&VideoFormat::ALL.map(VideoFormat::label), 0, window, cx);
        let preset_select = select_state(&Preset::labels(), 0, window, cx);
        let quality = slider_state(1., 100., DEFAULT_QUALITY, cx);

        let subscriptions = vec![
            cx.subscribe(
                &format_select,
                |this, _, event: &SelectEvent<SearchableVec<&'static str>>, cx| {
                    let SelectEvent::Confirm(label) = event;
                    if let Some(format) = label.and_then(VideoFormat::from_label) {
                        this.format = format;
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &preset_select,
                window,
                |this, _, event: &SelectEvent<SearchableVec<&'static str>>, window, cx| {
                    let SelectEvent::Confirm(label) = event;
                    if let Some(preset) = label.and_then(Preset::from_label) {
                        this.change_preset(preset, window, cx);
                    }
                },
            ),
            cx.subscribe(&quality, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        let mut page = Self {
            files: Vec::new(),
            selection: Selection::default(),
            format: VideoFormat::Mp4,
            format_select,
            preset: Preset::Original,
            preset_select,
            width: dimension_input(window, cx),
            height: dimension_input(window, cx),
            quality,
            output_dir: default_output_dir(),
            ffmpeg: Ffmpeg::Preparing,
            pending: Vec::new(),
            converting: false,
            progress: 0,
            results: Vec::new(),
            errors: Vec::new(),
            notice: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        page.prepare_ffmpeg(window, cx);
        page
    }

    /// Downloads ffmpeg on first use, which can take long enough to freeze the
    /// UI if it ran on the foreground thread.
    fn prepare_ffmpeg(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.ffmpeg = Ffmpeg::Preparing;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let outcome = cx
                .background_spawn(async { core_video::ensure_ffmpeg() })
                .await;
            this.update_in(cx, |this, window, cx| {
                this.ffmpeg = match outcome {
                    Ok(true) => Ffmpeg::Ready,
                    Ok(false) => Ffmpeg::Failed(FFMPEG_UNAVAILABLE_MESSAGE.into()),
                    Err(message) => Ffmpeg::Failed(message.into()),
                };
                let pending = std::mem::take(&mut this.pending);
                this.add_paths(pending, window, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn set_dimensions(
        &mut self,
        (width, height): (u32, u32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.width.update(cx, |input, cx| {
            input.set_value(width.to_string(), window, cx)
        });
        self.height.update(cx, |input, cx| {
            input.set_value(height.to_string(), window, cx)
        });
    }

    fn first_dimensions(&self) -> Option<(u32, u32)> {
        self.files.first().map(|f| (f.width, f.height))
    }

    fn change_preset(&mut self, preset: Preset, window: &mut Window, cx: &mut Context<Self>) {
        self.preset = preset;
        if let Some(dimensions) = preset_change_fill(preset, self.first_dimensions()) {
            self.set_dimensions(dimensions, window, cx);
        }
        cx.notify();
    }

    /// Paths from a picker or a drop are all accepted; files that are not
    /// readable videos surface in the error list instead of being filtered.
    pub fn add_paths(&mut self, paths: Vec<PathBuf>, window: &Window, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        if self.ffmpeg == Ffmpeg::Preparing {
            self.pending.extend(paths);
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            let loaded = cx
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(
                            |path| match core_video::get_video_info(&path.to_string_lossy()) {
                                Ok(info) => Loaded::File(VideoFile {
                                    name: info.name.into(),
                                    width: info.width,
                                    height: info.height,
                                    dimensions: format!("{}×{}", info.width, info.height).into(),
                                    duration: format_duration(info.duration_secs).into(),
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
            this.update_in(cx, |this, window, cx| {
                for item in loaded {
                    match item {
                        Loaded::File(file) => {
                            if this.files.is_empty() {
                                let source = (file.width, file.height);
                                this.set_dimensions(
                                    preset_dimensions(this.preset, source),
                                    window,
                                    cx,
                                );
                            }
                            this.files.push(file);
                        }
                        Loaded::Failed(error) => this.errors.push(error),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn pick_files(&mut self, window: &Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                this.update_in(cx, |this, window, cx| this.add_paths(paths, window, cx))
                    .ok();
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

    fn resize(&self, cx: &Context<Self>) -> Result<(Option<u32>, Option<u32>), &'static str> {
        let custom = if self.preset == Preset::Custom {
            parse_resize(&self.width.read(cx).value(), &self.height.read(cx).value())?
        } else {
            (None, None)
        };
        Ok(resize_for(self.preset, custom, self.first_dimensions()))
    }

    fn convert(&mut self, cx: &mut Context<Self>) {
        if cta_disabled(self.files.len(), self.converting, &self.ffmpeg) {
            return;
        }
        let (resize_width, resize_height) = match self.resize(cx) {
            Ok(resize) => resize,
            Err(message) => {
                self.notice = Some(message.into());
                cx.notify();
                return;
            }
        };

        self.notice = None;
        self.converting = true;
        self.progress = 0;
        self.results.clear();
        self.errors.clear();
        cx.notify();

        let jobs: Vec<(PathBuf, SharedString)> = self
            .files
            .iter()
            .map(|f| (f.path.clone(), f.name.clone()))
            .collect();
        let format = self.format;
        let output_dir = self.output_dir.clone();
        let quality = self.quality.read(cx).value().end().round() as u8;

        cx.spawn(async move |this, cx| {
            for (path, name) in jobs {
                let started = this.update(cx, |this, cx| {
                    this.progress = 0;
                    cx.notify();
                });
                if started.is_err() {
                    return;
                }

                let outcome = match precheck(&path, &output_dir, format) {
                    Err(message) => Err(message.to_string()),
                    Ok(()) => {
                        let request = ConvertVideoRequest {
                            source_path: path.to_string_lossy().into_owned(),
                            output_format: format.extension().to_string(),
                            output_dir: output_dir.to_string_lossy().into_owned(),
                            quality: Some(quality),
                            resize_width,
                            resize_height,
                        };
                        // core reports progress from inside its blocking ffmpeg
                        // loop; the channel carries it to this foreground task.
                        let (tx, mut rx) = mpsc::unbounded::<f64>();
                        let task = cx.background_spawn(async move {
                            core_video::convert_video(request, move |progress| {
                                tx.unbounded_send(progress.percent).ok();
                            })
                        });
                        while let Some(percent) = rx.next().await {
                            let updated = this.update(cx, |this, cx| {
                                this.progress = percent_of(percent);
                                cx.notify();
                            });
                            if updated.is_err() {
                                return;
                            }
                        }
                        task.await
                    }
                };

                let updated = this.update(cx, |this, cx| {
                    match outcome {
                        Ok(result) => {
                            let output = PathBuf::from(&result.output_path);
                            this.results.push(Converted {
                                name: file_name(&output),
                                detail: format!(
                                    "{} → {}",
                                    format_size(result.original_size),
                                    format_size(result.output_size)
                                )
                                .into(),
                                path: output,
                            });
                            this.files.retain(|f| f.path != path);
                            this.selection
                                .retain_present(this.files.iter().map(|f| f.path.as_path()));
                        }
                        Err(message) => this.errors.push(ConvertError {
                            name,
                            path,
                            message: message.into(),
                        }),
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
            this.update(cx, |this, cx| {
                // The batch status only reports a finished run once progress
                // is non-zero, which a batch of failures never reaches.
                this.progress = 100;
                this.converting = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn rows(&self) -> Vec<FileRow> {
        self.files
            .iter()
            .map(|file| FileRow {
                path: file.path.clone(),
                name: file.name.clone(),
                thumb: Thumb::None,
                selected: self.selection.contains(&file.path),
                cells: vec![
                    text_cell(file.dimensions.clone()),
                    text_cell(file.duration.clone()),
                    text_cell(file.size.clone()),
                ],
            })
            .collect()
    }

    fn notice(&self, t: Tokens, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pill = |bg, fg| {
            div()
                .self_start()
                .flex()
                .items_center()
                .gap(px(12.))
                .py(px(8.))
                .px(px(14.))
                .rounded(px(RADIUS_SM))
                .bg(bg)
                .text_size(px(13.))
                .text_color(fg)
        };
        if let Some(message) = self.notice.clone() {
            return Some(
                pill(t.error_tint, t.error)
                    .child(message)
                    .into_any_element(),
            );
        }
        match &self.ffmpeg {
            Ffmpeg::Ready => None,
            Ffmpeg::Preparing => Some(
                pill(t.accent_tint, t.accent)
                    .child("ffmpeg を準備中...")
                    .into_any_element(),
            ),
            Ffmpeg::Failed(message) => {
                let this = cx.entity();
                Some(
                    pill(t.error_tint, t.error)
                        .child(message.clone())
                        .child(
                            div()
                                .id("ffmpeg-retry")
                                .cursor_pointer()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .underline()
                                .child("再試行")
                                .on_click(move |_, window, cx| {
                                    this.update(cx, |this, cx| this.prepare_ffmpeg(window, cx))
                                }),
                        )
                        .into_any_element(),
                )
            }
        }
    }
}

impl Render for VideoPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let this = cx.entity();
        let quality = self.quality.read(cx).value().end().round() as u32;

        let status = batch_status(BatchState {
            file_count: self.files.len(),
            converting: self.converting,
            progress: self.progress,
            result_count: self.results.len(),
            error_count: self.errors.len(),
        });

        let header_state = self
            .selection
            .state_over(self.files.iter().map(|f| f.path.as_path()));
        let columns = vec![
            Column::thumb(IconName::Video),
            Column::name(),
            Column::cell("解像度", 88.),
            Column::cell("長さ", 52.),
            Column::cell("ファイルサイズ", 96.),
        ];
        let table = FileTable::new(self.rows(), columns, header_state, t)
            .empty(
                IconName::Video,
                "ここに動画ファイルをドロップまたはクリックして選択",
                "MP4, MOV, AVI, WebM, FLV, HEVC",
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
                move |window, cx| this.update(cx, |this, cx| this.pick_files(window, cx))
            })
            .on_drop({
                let this = this.clone();
                move |paths, window, cx| {
                    this.update(cx, |this, cx| this.add_paths(paths, window, cx))
                }
            });

        let outputs = self
            .results
            .iter()
            .map(|r| OutputItem {
                name: r.name.clone(),
                detail: r.detail.clone(),
                path: r.path.clone(),
                full_path: None,
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

        let resolution_field = Field::new("解像度", t)
            .child(themed_select(&self.preset_select, t))
            .when(self.preset.shows_dimensions(), |field| {
                field.child(
                    DimensionRow::new(&self.width, &self.height, t)
                        .disabled(!self.preset.dimensions_editable()),
                )
            });

        div()
            .size_full()
            .child(
                PageShell::new(Page::Video.title(), "書き出し設定", &self.scroll, t)
                    .notice(self.notice(t, cx))
                    .toolbar(
                        Toolbar::new(t)
                            .delete_disabled(self.selection.count() == 0)
                            .on_add({
                                let this = this.clone();
                                move |window, cx| {
                                    this.update(cx, |this, cx| this.pick_files(window, cx))
                                }
                            })
                            .on_delete({
                                let this = this.clone();
                                move |_, cx| this.update(cx, |this, cx| this.remove_selected(cx))
                            }),
                    )
                    .field(Field::new("出力形式", t).child(themed_select(&self.format_select, t)))
                    .field(resolution_field)
                    .field(
                        Field::new("画質", t)
                            .aside(ValueBadge::new(format!("{quality}%"), t))
                            .child(themed_slider(&self.quality, false)),
                    )
                    .field(Field::new("保存先", t).child({
                        let this = this.clone();
                        PathPicker::new(&self.output_dir, t).on_pick(move |_, cx| {
                            this.update(cx, |this, cx| this.pick_output_dir(cx))
                        })
                    }))
                    .footer(
                        Footer::new(status, self.progress, "変換する", t)
                            .cta_disabled(cta_disabled(
                                self.files.len(),
                                self.converting,
                                &self.ffmpeg,
                            ))
                            .on_cta({
                                let this = this.clone();
                                move |_, cx| this.update(cx, |this, cx| this.convert(cx))
                            }),
                    )
                    .child(table)
                    .child(ResultList::new(outputs, errors, t)),
            )
            .when(self.converting, |this| {
                this.child(PixelLoader::new("動画変換中...").progress(self.progress))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_HD: (u32, u32) = (1920, 1080);

    #[test]
    fn preset_labels_round_trip() {
        for (label, preset) in Preset::ALL {
            assert_eq!(Preset::from_label(label), Some(preset));
        }
        assert_eq!(Preset::from_label("4K"), None);
    }

    #[test]
    fn preset_labels_are_offered_in_the_original_order() {
        assert_eq!(
            Preset::labels(),
            ["オリジナル", "1080p", "720p", "480p", "カスタム"]
        );
    }

    #[test]
    fn dimension_fields_are_hidden_only_for_the_original_preset() {
        assert!(!Preset::Original.shows_dimensions());
        assert!(Preset::Height(720).shows_dimensions());
        assert!(Preset::Custom.shows_dimensions());
    }

    #[test]
    fn dimension_fields_are_editable_only_for_the_custom_preset() {
        assert!(Preset::Custom.dimensions_editable());
        assert!(!Preset::Height(720).dimensions_editable());
        assert!(!Preset::Original.dimensions_editable());
    }

    #[test]
    fn format_labels_round_trip() {
        for format in VideoFormat::ALL {
            assert_eq!(VideoFormat::from_label(format.label()), Some(format));
        }
        assert_eq!(VideoFormat::from_label("MKV"), None);
    }

    #[test]
    fn format_extension_is_the_lowercase_label_core_accepts() {
        for format in VideoFormat::ALL {
            assert_eq!(format.extension(), format.label().to_lowercase());
        }
    }

    #[test]
    fn original_preset_requests_no_resize() {
        assert_eq!(
            resize_for(Preset::Original, (Some(10), Some(10)), Some(FULL_HD)),
            (None, None)
        );
    }

    #[test]
    fn custom_preset_requests_the_typed_values() {
        assert_eq!(
            resize_for(Preset::Custom, (Some(640), None), Some(FULL_HD)),
            (Some(640), None)
        );
    }

    #[test]
    fn numeric_preset_sets_height_and_derives_width_from_the_first_file() {
        assert_eq!(
            resize_for(Preset::Height(720), (None, None), Some(FULL_HD)),
            (Some(1280), Some(720))
        );
        assert_eq!(
            resize_for(Preset::Height(480), (None, None), Some((1000, 600))),
            (Some(800), Some(480))
        );
    }

    #[test]
    fn numeric_preset_without_a_file_leaves_the_width_unset() {
        assert_eq!(
            resize_for(Preset::Height(720), (None, None), None),
            (None, Some(720))
        );
    }

    #[test]
    fn derived_width_is_rounded_to_the_nearest_pixel() {
        assert_eq!(scaled_width(480, (1000, 998)), Some(481));
        assert_eq!(scaled_width(720, (1, 3)), Some(240));
        assert_eq!(scaled_width(720, (1, 1080)), Some(1));
    }

    #[test]
    fn zero_height_source_has_no_aspect_ratio() {
        assert_eq!(scaled_width(720, (1920, 0)), None);
        assert_eq!(
            preset_dimensions(Preset::Height(720), (1920, 0)),
            (1920, 720)
        );
    }

    #[test]
    fn picking_a_numeric_preset_fills_dimensions_from_the_first_file() {
        assert_eq!(
            preset_change_fill(Preset::Height(1080), Some((1280, 720))),
            Some((1920, 1080))
        );
    }

    #[test]
    fn picking_original_fills_the_first_file_dimensions() {
        assert_eq!(
            preset_change_fill(Preset::Original, Some(FULL_HD)),
            Some(FULL_HD)
        );
    }

    #[test]
    fn picking_custom_keeps_the_typed_values() {
        assert_eq!(preset_change_fill(Preset::Custom, Some(FULL_HD)), None);
    }

    #[test]
    fn picking_a_preset_without_files_fills_nothing() {
        assert_eq!(preset_change_fill(Preset::Height(720), None), None);
        assert_eq!(preset_change_fill(Preset::Original, None), None);
    }

    #[test]
    fn convert_needs_files_an_idle_page_and_a_ready_ffmpeg() {
        assert!(!cta_disabled(1, false, &Ffmpeg::Ready));
        assert!(cta_disabled(0, false, &Ffmpeg::Ready));
        assert!(cta_disabled(1, true, &Ffmpeg::Ready));
    }

    #[test]
    fn convert_is_blocked_until_ffmpeg_is_ready() {
        assert!(cta_disabled(1, false, &Ffmpeg::Preparing));
        assert!(cta_disabled(1, false, &Ffmpeg::Failed("x".into())));
    }

    #[test]
    fn ffmpeg_percent_is_rounded_and_clamped() {
        assert_eq!(percent_of(0.0), 0);
        assert_eq!(percent_of(42.5), 43);
        assert_eq!(percent_of(100.0), 100);
        assert_eq!(percent_of(130.0), 100);
        assert_eq!(percent_of(-1.0), 0);
    }

    #[test]
    fn converting_onto_the_source_file_is_rejected() {
        assert_eq!(
            precheck(
                Path::new("/in/clip.mp4"),
                Path::new("/in"),
                VideoFormat::Mp4
            ),
            Err("出力先が入力ファイルと同じです")
        );
    }

    #[test]
    fn converting_to_another_format_in_the_same_dir_is_allowed() {
        assert_eq!(
            precheck(
                Path::new("/in/clip.mp4"),
                Path::new("/in"),
                VideoFormat::Webm
            ),
            Ok(())
        );
    }
}
