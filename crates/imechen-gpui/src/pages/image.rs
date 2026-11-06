use std::path::{Path, PathBuf};

use gpui::{
    div, prelude::*, px, Context, Entity, PathPromptOptions, ScrollHandle, SharedString,
    Subscription, Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::SelectEvent;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use imechen_core::image::{self as core_image, ConvertImageRequest};

use super::Page;
use crate::components::{
    batch_status, dimension_input, parse_dimension, select_state, slider_state, text_cell,
    themed_select, themed_slider, BatchState, CheckState, Checkbox, Column, ConvertError,
    DimensionRow, Field, FileRow, FileTable, Footer, NoteText, OutputItem, PageShell, PathPicker,
    PixelLoader, ResultList, Selection, Thumb, Toolbar, ValueBadge,
};
use crate::media::format_size;
use crate::output::{is_same_file, target_path};
use crate::theme::{ActiveTokens as _, Tokens, RADIUS_SM};

const SAME_PATH_MESSAGE: &str = "出力先が入力ファイルと同じです";
const DEFAULT_QUALITY: f32 = 85.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
}

impl OutputFormat {
    const ALL: [Self; 5] = [Self::Png, Self::Jpeg, Self::Gif, Self::Webp, Self::Bmp];

    fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Gif => "GIF",
            Self::Webp => "WebP",
            Self::Bmp => "BMP",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.label() == label)
    }

    /// The format name core's request expects.
    fn request_name(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
        }
    }

    /// Must match the extension core gives its output file.
    fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            other => other.request_name(),
        }
    }

    fn supports_quality(self) -> bool {
        matches!(self, Self::Jpeg | Self::Webp | Self::Png)
    }
}

/// Resize fields are validated before the batch starts so a typo is reported
/// once, instead of as the same error on every file.
fn resize_request(width: &str, height: &str) -> Result<(Option<u32>, Option<u32>), &'static str> {
    let width = parse_dimension(width).map_err(|_| "幅は1以上の整数で指定してください")?;
    let height = parse_dimension(height).map_err(|_| "高さは1以上の整数で指定してください")?;
    Ok((width, height))
}

fn progress_percent(done: usize, total: usize) -> u32 {
    if total == 0 {
        return 0;
    }
    (done as f64 / total as f64 * 100.0).round() as u32
}

fn precheck(source: &Path, dir: &Path, format: OutputFormat) -> Result<(), &'static str> {
    if is_same_file(source, &target_path(source, dir, format.extension())) {
        Err(SAME_PATH_MESSAGE)
    } else {
        Ok(())
    }
}

fn default_output_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn file_name(path: &Path) -> SharedString {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
        .into()
}

struct ImageFile {
    path: PathBuf,
    name: SharedString,
    dimensions: SharedString,
    size: SharedString,
}

struct Converted {
    path: PathBuf,
    name: SharedString,
    detail: SharedString,
}

enum Loaded {
    File(ImageFile),
    Failed(ConvertError),
}

pub struct ImagePage {
    files: Vec<ImageFile>,
    selection: Selection,
    format: OutputFormat,
    format_select: Entity<crate::components::OptionSelect>,
    quality: Entity<SliderState>,
    width: Entity<InputState>,
    height: Entity<InputState>,
    keep_aspect: bool,
    output_dir: PathBuf,
    converting: bool,
    progress: u32,
    results: Vec<Converted>,
    errors: Vec<ConvertError>,
    notice: Option<SharedString>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl ImagePage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let labels = OutputFormat::ALL.map(OutputFormat::label);
        let format_select = select_state(&labels, 0, window, cx);
        let quality = slider_state(1., 100., DEFAULT_QUALITY, cx);

        let subscriptions = vec![
            cx.subscribe(
                &format_select,
                |this, _, event: &SelectEvent<SearchableVec<&'static str>>, cx| {
                    let SelectEvent::Confirm(label) = event;
                    if let Some(format) = label.and_then(OutputFormat::from_label) {
                        this.format = format;
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(&quality, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        Self {
            files: Vec::new(),
            selection: Selection::default(),
            format: OutputFormat::Png,
            format_select,
            quality,
            width: dimension_input(window, cx),
            height: dimension_input(window, cx),
            keep_aspect: true,
            output_dir: default_output_dir(),
            converting: false,
            progress: 0,
            results: Vec::new(),
            errors: Vec::new(),
            notice: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Paths from a picker or a drop are all accepted; files that are not
    /// readable images surface in the error list instead of being filtered.
    pub fn add_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(
                            |path| match core_image::get_image_info(&path.to_string_lossy()) {
                                Ok(info) => Loaded::File(ImageFile {
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
                        Loaded::File(file) => this.files.push(file),
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

    fn convert(&mut self, cx: &mut Context<Self>) {
        if self.converting || self.files.is_empty() {
            return;
        }
        let (resize_width, resize_height) =
            match resize_request(&self.width.read(cx).value(), &self.height.read(cx).value()) {
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
        let keep_aspect = self.keep_aspect;

        cx.spawn(async move |this, cx| {
            let total = jobs.len();
            for (i, (path, name)) in jobs.into_iter().enumerate() {
                let outcome = match precheck(&path, &output_dir, format) {
                    Err(message) => Err(message.to_string()),
                    Ok(()) => {
                        let request = ConvertImageRequest {
                            source_path: path.to_string_lossy().into_owned(),
                            output_format: format.request_name().to_string(),
                            output_dir: output_dir.to_string_lossy().into_owned(),
                            quality: Some(quality),
                            resize_width,
                            resize_height,
                            keep_aspect_ratio: Some(keep_aspect),
                        };
                        cx.background_spawn(async move { core_image::convert_image(request) })
                            .await
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
                    this.progress = progress_percent(i + 1, total);
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
            this.update(cx, |this, cx| {
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
                thumb: Thumb::Path(file.path.clone()),
                selected: self.selection.contains(&file.path),
                cells: vec![
                    text_cell(file.dimensions.clone()),
                    text_cell(file.size.clone()),
                ],
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

impl Render for ImagePage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let this = cx.entity();
        let can_quality = self.format.supports_quality();
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
            Column::thumb(IconName::Image),
            Column::name(),
            Column::cell("画像サイズ", 92.),
            Column::cell("ファイルサイズ", 96.),
        ];
        let table = FileTable::new(self.rows(), columns, header_state, t)
            .empty(
                IconName::Image,
                "ここにファイルをドロップまたはクリックして選択",
                "JPEG, PNG, GIF, WebP, BMP, HEIF",
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

        let resize_field = Field::new("リサイズ（任意）", t)
            .child(DimensionRow::new(&self.width, &self.height, t))
            .child(
                Checkbox::new(
                    "keep-aspect",
                    if self.keep_aspect {
                        CheckState::Checked
                    } else {
                        CheckState::Unchecked
                    },
                    t,
                )
                .label("縦横比を維持")
                .on_toggle({
                    let this = this.clone();
                    move |_, cx| {
                        this.update(cx, |this, cx| {
                            this.keep_aspect = !this.keep_aspect;
                            cx.notify();
                        })
                    }
                }),
            );

        let quality_field = Field::new("品質", t)
            .dimmed(!can_quality)
            .aside(if can_quality {
                ValueBadge::new(format!("{quality}%"), t).into_any_element()
            } else {
                NoteText::new(
                    format!("{} は画質設定非対応", self.format.label().to_uppercase()),
                    t,
                )
                .into_any_element()
            })
            .child(themed_slider(&self.quality, !can_quality));

        div()
            .size_full()
            .child(
                PageShell::new(Page::Image.title(), "書き出し設定", &self.scroll, t)
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
                    .field(resize_field)
                    .field(quality_field)
                    .field(Field::new("保存先", t).child({
                        let this = this.clone();
                        PathPicker::new(&self.output_dir, t).on_pick(move |_, cx| {
                            this.update(cx, |this, cx| this.pick_output_dir(cx))
                        })
                    }))
                    .footer(
                        Footer::new(status, self.progress, "変換する", t)
                            .cta_disabled(self.files.is_empty() || self.converting)
                            .on_cta({
                                let this = this.clone();
                                move |_, cx| this.update(cx, |this, cx| this.convert(cx))
                            }),
                    )
                    .child(table)
                    .child(ResultList::new(outputs, errors, t)),
            )
            .when(self.converting, |this| {
                this.child(PixelLoader::new("変換中...").progress(self.progress))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_applies_to_png_jpeg_and_webp_only() {
        assert!(OutputFormat::Png.supports_quality());
        assert!(OutputFormat::Jpeg.supports_quality());
        assert!(OutputFormat::Webp.supports_quality());
        assert!(!OutputFormat::Gif.supports_quality());
        assert!(!OutputFormat::Bmp.supports_quality());
    }

    #[test]
    fn format_labels_round_trip() {
        for format in OutputFormat::ALL {
            assert_eq!(OutputFormat::from_label(format.label()), Some(format));
        }
        assert_eq!(OutputFormat::from_label("TIFF"), None);
    }

    #[test]
    fn jpeg_output_uses_the_jpg_extension_like_core() {
        assert_eq!(OutputFormat::Jpeg.request_name(), "jpeg");
        assert_eq!(OutputFormat::Jpeg.extension(), "jpg");
        assert_eq!(OutputFormat::Webp.extension(), "webp");
    }

    #[test]
    fn empty_resize_fields_request_no_resize() {
        assert_eq!(resize_request("", ""), Ok((None, None)));
    }

    #[test]
    fn single_resize_field_leaves_the_other_unset() {
        assert_eq!(resize_request("800", ""), Ok((Some(800), None)));
        assert_eq!(resize_request("", "600"), Ok((None, Some(600))));
    }

    #[test]
    fn invalid_resize_value_names_the_offending_field() {
        assert_eq!(
            resize_request("0", "10"),
            Err("幅は1以上の整数で指定してください")
        );
        assert_eq!(
            resize_request("10", "x"),
            Err("高さは1以上の整数で指定してください")
        );
    }

    #[test]
    fn progress_is_rounded_to_the_nearest_percent() {
        assert_eq!(progress_percent(1, 3), 33);
        assert_eq!(progress_percent(2, 3), 67);
        assert_eq!(progress_percent(3, 3), 100);
        assert_eq!(progress_percent(0, 0), 0);
    }

    #[test]
    fn converting_onto_the_source_file_is_rejected() {
        let source = Path::new("/in/photo.png");
        assert_eq!(
            precheck(source, Path::new("/in"), OutputFormat::Png),
            Err("出力先が入力ファイルと同じです")
        );
    }

    #[test]
    fn converting_to_another_format_in_the_same_dir_is_allowed() {
        let source = Path::new("/in/photo.png");
        assert_eq!(
            precheck(source, Path::new("/in"), OutputFormat::Webp),
            Ok(())
        );
    }

    #[test]
    fn jpeg_to_jpg_source_in_the_same_dir_is_rejected() {
        let source = Path::new("/in/photo.jpg");
        assert!(precheck(source, Path::new("/in"), OutputFormat::Jpeg).is_err());
    }
}
