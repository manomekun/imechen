use std::path::{Path, PathBuf};

use futures::{channel::mpsc, StreamExt};
use gpui::{
    div, img, prelude::*, px, relative, AsyncApp, Context, Div, ExternalPaths, ObjectFit,
    PathPromptOptions, SharedString, Stateful, Subscription, WeakEntity, Window,
};
use imechen_core::image::{self as core_image, ConvertImageRequest};
use imechen_core::video::{self as core_video, ConvertVideoRequest};

use crate::media::{classify, format_duration, human_size, MediaKind};
use crate::theme::Palette;

enum Info {
    Loading,
    Image {
        width: u32,
        height: u32,
        size: u64,
    },
    Video {
        width: u32,
        height: u32,
        size: u64,
        duration_secs: f64,
        codec: String,
    },
    Failed(String),
}

enum Convert {
    Idle,
    Queued,
    Running(f64),
    Done { file_name: String, output_size: u64 },
    Failed(String),
}

struct Row {
    path: PathBuf,
    kind: Option<MediaKind>,
    info: Info,
    convert: Convert,
}

enum Ffmpeg {
    Preparing,
    Ready,
    Failed(String),
}

pub struct Imechen {
    rows: Vec<Row>,
    output_dir: Option<PathBuf>,
    ffmpeg: Ffmpeg,
    converting: bool,
    palette: Palette,
    _appearance_subscription: Subscription,
}

impl Imechen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_window_appearance(window, |this, window, cx| {
            this.palette = Palette::for_appearance(window.appearance());
            cx.notify();
        });

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async { core_video::ensure_ffmpeg() })
                .await;
            this.update(cx, |this, cx| {
                this.ffmpeg = match result {
                    Ok(_) => Ffmpeg::Ready,
                    Err(e) => Ffmpeg::Failed(e),
                };
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            rows: Vec::new(),
            output_dir: None,
            ffmpeg: Ffmpeg::Preparing,
            converting: false,
            palette: Palette::for_appearance(window.appearance()),
            _appearance_subscription: subscription,
        }
    }

    pub fn add_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        for path in paths {
            if self.rows.iter().any(|r| r.path == path) {
                continue;
            }
            let kind = classify(&path);
            self.rows.push(Row {
                path: path.clone(),
                kind,
                info: match kind {
                    Some(_) => Info::Loading,
                    None => Info::Failed("未対応の形式".into()),
                },
                convert: Convert::Idle,
            });
            if let Some(kind) = kind {
                self.load_info(path, kind, cx);
            }
        }
        if self.output_dir.is_none() {
            self.output_dir = self
                .rows
                .first()
                .and_then(|r| r.path.parent())
                .map(Path::to_path_buf);
        }
        cx.notify();
    }

    fn load_info(&mut self, path: PathBuf, kind: MediaKind, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let target = path.to_string_lossy().into_owned();
            let info = cx
                .background_spawn(async move {
                    match kind {
                        MediaKind::Image => {
                            core_image::get_image_info(&target).map(|i| Info::Image {
                                width: i.width,
                                height: i.height,
                                size: i.size,
                            })
                        }
                        MediaKind::Video => {
                            core_video::get_video_info(&target).map(|v| Info::Video {
                                width: v.width,
                                height: v.height,
                                size: v.size,
                                duration_secs: v.duration_secs,
                                codec: v.codec,
                            })
                        }
                    }
                })
                .await;
            Self::update_row(&this, cx, &path, |row| {
                row.info = info.unwrap_or_else(Info::Failed);
            });
        })
        .detach();
    }

    fn update_row(
        this: &WeakEntity<Self>,
        cx: &mut AsyncApp,
        path: &Path,
        f: impl FnOnce(&mut Row),
    ) {
        this.update(cx, |this, cx| {
            if let Some(row) = this.rows.iter_mut().find(|r| r.path == path) {
                f(row);
            }
            cx.notify();
        })
        .ok();
    }

    fn pick_files(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                this.update(cx, |this, cx| this.add_paths(paths, cx)).ok();
            }
        })
        .detach();
    }

    fn pick_output_dir(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = rx.await {
                if let Some(dir) = paths.pop() {
                    this.update(cx, |this, cx| {
                        this.output_dir = Some(dir);
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn start_conversion(&mut self, cx: &mut Context<Self>) {
        if self.converting {
            return;
        }
        let Some(output_dir) = self.output_dir.clone() else {
            return;
        };
        let mut jobs = Vec::new();
        for row in &mut self.rows {
            let ready = matches!(row.info, Info::Image { .. } | Info::Video { .. });
            match row.kind {
                Some(kind) if ready => {
                    row.convert = Convert::Queued;
                    jobs.push((row.path.clone(), kind));
                }
                _ => row.convert = Convert::Idle,
            }
        }
        if jobs.is_empty() {
            return;
        }
        self.converting = true;
        cx.notify();

        let output_dir = output_dir.to_string_lossy().into_owned();
        cx.spawn(async move |this, cx| {
            for (path, kind) in jobs {
                // The default output dir is the source's own dir, so e.g. a.mp4 -> a.mp4
                // would overwrite the original in place.
                let target_ext = match kind {
                    MediaKind::Image => "webp",
                    MediaKind::Video => "mp4",
                };
                if Path::new(&output_dir).join(
                    path.with_extension(target_ext)
                        .file_name()
                        .unwrap_or_default(),
                ) == path
                {
                    Self::update_row(&this, cx, &path, |r| {
                        r.convert = Convert::Failed("出力先が入力ファイルと同じです".into());
                    });
                    continue;
                }
                let outcome = match kind {
                    MediaKind::Image => {
                        let request = ConvertImageRequest {
                            source_path: path.to_string_lossy().into_owned(),
                            output_format: "webp".into(),
                            output_dir: output_dir.clone(),
                            quality: Some(80),
                            resize_width: None,
                            resize_height: None,
                            keep_aspect_ratio: None,
                        };
                        Self::update_row(&this, cx, &path, |r| r.convert = Convert::Running(0.0));
                        cx.background_spawn(async move { core_image::convert_image(request) })
                            .await
                            .map(|r| (r.output_path, r.output_size))
                    }
                    MediaKind::Video => {
                        let request = ConvertVideoRequest {
                            source_path: path.to_string_lossy().into_owned(),
                            output_format: "mp4".into(),
                            output_dir: output_dir.clone(),
                            quality: None,
                            resize_width: None,
                            resize_height: None,
                        };
                        let (tx, mut rx) = mpsc::unbounded::<f64>();
                        // The blocking ffmpeg loop cannot touch entities, so progress
                        // is relayed over a channel that this foreground task drains.
                        let task = cx.background_spawn(async move {
                            core_video::convert_video(request, move |p| {
                                let _ = tx.unbounded_send(p.percent);
                            })
                        });
                        Self::update_row(&this, cx, &path, |r| r.convert = Convert::Running(0.0));
                        while let Some(percent) = rx.next().await {
                            Self::update_row(&this, cx, &path, |r| {
                                r.convert = Convert::Running(percent)
                            });
                        }
                        task.await.map(|r| (r.output_path, r.output_size))
                    }
                };
                Self::update_row(&this, cx, &path, |r| {
                    r.convert = match outcome {
                        Ok((output_path, output_size)) => Convert::Done {
                            file_name: Path::new(&output_path)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or(output_path),
                            output_size,
                        },
                        Err(e) => Convert::Failed(e),
                    };
                });
            }
            this.update(cx, |this, cx| {
                this.converting = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn overall_progress(&self) -> Option<(f32, usize, usize)> {
        let batch: Vec<&Row> = self
            .rows
            .iter()
            .filter(|r| !matches!(r.convert, Convert::Idle))
            .collect();
        if batch.is_empty() {
            return None;
        }
        let mut done = 0;
        let mut sum = 0.0;
        for row in &batch {
            match row.convert {
                Convert::Done { .. } | Convert::Failed(_) => {
                    done += 1;
                    sum += 1.0;
                }
                Convert::Running(p) => sum += (p / 100.0) as f32,
                Convert::Queued | Convert::Idle => {}
            }
        }
        Some((sum / batch.len() as f32, done, batch.len()))
    }
}

fn button(id: &'static str, label: &'static str, p: &Palette, enabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .px_4()
        .py_2()
        .rounded_md()
        .text_sm()
        .bg(if enabled { p.accent } else { p.disabled })
        .text_color(p.accent_text)
        .when(enabled, |b| b.cursor_pointer())
        .child(label)
}

fn progress_bar(fraction: f32, p: &Palette) -> Div {
    div()
        .h(px(8.))
        .w_full()
        .rounded_full()
        .bg(p.border)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(relative(fraction.clamp(0.0, 1.0)))
                .rounded_full()
                .bg(p.accent),
        )
}

fn render_row(ix: usize, row: &Row, p: &Palette) -> Stateful<Div> {
    let name: SharedString = row
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
        .into();

    let preview = match row.kind {
        Some(MediaKind::Image) => img(row.path.clone())
            .size_full()
            .object_fit(ObjectFit::Contain)
            .into_any_element(),
        _ => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(p.text_muted)
            .child(if row.kind.is_some() { "VIDEO" } else { "?" })
            .into_any_element(),
    };

    let (info_text, info_color) = match &row.info {
        Info::Loading => ("読み込み中…".to_string(), p.text_muted),
        Info::Image {
            width,
            height,
            size,
        } => (
            format!("{width}×{height}  {}", human_size(*size)),
            p.text_muted,
        ),
        Info::Video {
            width,
            height,
            size,
            duration_secs,
            codec,
        } => (
            format!(
                "{width}×{height}  {}  {codec}  {}",
                format_duration(*duration_secs),
                human_size(*size)
            ),
            p.text_muted,
        ),
        Info::Failed(e) => (format!("エラー: {e}"), p.danger),
    };

    let status = match &row.convert {
        Convert::Idle => None,
        Convert::Queued => Some(div().text_xs().text_color(p.text_muted).child("待機中")),
        Convert::Running(percent) => Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .child(progress_bar((*percent / 100.0) as f32, p)),
                )
                .child(div().w(px(44.)).text_xs().child(format!("{percent:.0}%"))),
        ),
        Convert::Done {
            file_name,
            output_size,
        } => Some(
            div()
                .text_xs()
                .text_color(p.success)
                .child(format!("完了: {file_name} ({})", human_size(*output_size))),
        ),
        Convert::Failed(e) => Some(
            div()
                .text_xs()
                .text_color(p.danger)
                .child(format!("変換エラー: {e}")),
        ),
    };

    div()
        .id(("row", ix))
        .flex()
        .gap_3()
        .p_2()
        .rounded_md()
        .bg(p.surface)
        .border_1()
        .border_color(p.border)
        .child(
            div()
                .size_16()
                .flex_shrink_0()
                .rounded_sm()
                .overflow_hidden()
                .bg(p.bg)
                .child(preview),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().truncate().child(name))
                .child(div().text_xs().text_color(info_color).child(info_text))
                .children(status),
        )
}

impl Render for Imechen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let can_convert = !self.converting && self.output_dir.is_some() && !self.rows.is_empty();

        let (ffmpeg_text, ffmpeg_color) = match &self.ffmpeg {
            Ffmpeg::Preparing => ("ffmpeg: 準備中".to_string(), p.text_muted),
            Ffmpeg::Ready => ("ffmpeg: OK".to_string(), p.success),
            Ffmpeg::Failed(e) => (format!("ffmpeg: エラー: {e}"), p.danger),
        };
        let output_text = self
            .output_dir
            .as_ref()
            .map(|d| format!("出力先: {}", d.display()))
            .unwrap_or_else(|| "出力先: 未選択".to_string());

        let overall = self.overall_progress().map(|(fraction, done, total)| {
            div()
                .flex()
                .items_center()
                .gap_3()
                .text_xs()
                .child(div().flex_1().child(progress_bar(fraction, &p)))
                .child(format!("{done}/{total}  {:.0}%", fraction * 100.0))
        });

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(p.bg)
            .text_color(p.text)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().child("imechen"))
                    .child(div().text_xs().text_color(ffmpeg_color).child(ffmpeg_text)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        button("pick-files", "ファイルを選択", &p, true)
                            .on_click(cx.listener(|this, _, _, cx| this.pick_files(cx))),
                    )
                    .child(
                        button("pick-dir", "出力先を選択", &p, true)
                            .on_click(cx.listener(|this, _, _, cx| this.pick_output_dir(cx))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(p.text_muted)
                            .child(output_text),
                    )
                    .child(
                        button("convert", "変換", &p, can_convert)
                            .when(can_convert, |b| {
                                b.on_click(cx.listener(|this, _, _, cx| this.start_conversion(cx)))
                            }),
                    ),
            )
            .child(
                div()
                    .id("drop-zone")
                    .h_20()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .border_2()
                    .border_dashed()
                    .border_color(p.border)
                    .text_sm()
                    .text_color(p.text_muted)
                    .drag_over::<ExternalPaths>(move |style, _, _, _| {
                        style.bg(p.drop_active).border_color(p.accent)
                    })
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                        this.add_paths(paths.paths().to_vec(), cx);
                    }))
                    .child("ここにファイルをドロップ (画像: jpg/png/gif/webp/bmp, 動画: mp4/mov/avi/webm/flv/mkv)"),
            )
            .children(overall)
            .child(
                div()
                    .id("file-list")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .overflow_y_scroll()
                    .children(
                        self.rows
                            .iter()
                            .enumerate()
                            .map(|(ix, row)| render_row(ix, row, &p)),
                    ),
            )
    }
}
