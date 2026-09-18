use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{Emitter, AppHandle};

#[derive(Debug, Deserialize)]
pub struct ConvertVideoRequest {
    pub source_path: String,
    pub output_format: String,
    pub output_dir: String,
    pub quality: Option<u8>,
    pub resize_width: Option<u32>,
    pub resize_height: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ConvertVideoResult {
    pub output_path: String,
    pub original_size: u64,
    pub output_size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct VideoInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,
    pub format: String,
    pub codec: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VideoProgress {
    pub percent: f64,
    pub fps: f32,
    pub speed: String,
}

/// Parse ffmpeg time string "HH:MM:SS.ms" to seconds
fn parse_ffmpeg_time(time_str: &str) -> f64 {
    let parts: Vec<&str> = time_str.split(':').collect();
    match parts.as_slice() {
        [h, m, s] => {
            let hours: f64 = h.parse().unwrap_or(0.0);
            let mins: f64 = m.parse().unwrap_or(0.0);
            let secs: f64 = s.parse().unwrap_or(0.0);
            hours * 3600.0 + mins * 60.0 + secs
        }
        _ => 0.0,
    }
}

fn format_extension(format: &str) -> &str {
    match format.to_lowercase().as_str() {
        "mp4" => "mp4",
        "mov" => "mov",
        "avi" => "avi",
        "webm" => "webm",
        "flv" => "flv",
        _ => "mp4",
    }
}

fn get_video_codec_for_format(format: &str) -> &str {
    match format.to_lowercase().as_str() {
        "mp4" => "libx264",
        "mov" => "libx264",
        "avi" => "libx264",
        "webm" => "libvpx-vp9",
        "flv" => "libx264",
        _ => "libx264",
    }
}

/// Read input metadata from ffmpeg's own stderr instead of ffprobe:
/// the macOS sidecar download ships only the ffmpeg binary, so ffprobe
/// cannot be assumed to exist next to it.
fn probe_video(path: &str) -> Result<(u32, u32, f64, String, String), String> {
    let iter = FfmpegCommand::new()
        .input(path)
        .spawn()
        .map_err(|e| format!("ffmpeg spawn failed: {}", e))?
        .iter()
        .map_err(|e| format!("ffmpeg iter failed: {}", e))?;

    let mut format: Option<String> = None;
    let mut duration: Option<f64> = None;
    let mut video: Option<(u32, u32, String)> = None;

    for event in iter {
        match event {
            FfmpegEvent::ParsedInput(input) if input.index == 0 => {
                format = parse_input_format(&input.raw_log_message);
            }
            FfmpegEvent::ParsedDuration(d) if d.input_index == 0 => {
                duration = Some(d.duration);
            }
            FfmpegEvent::ParsedInputStream(stream)
                if stream.parent_index == 0 && video.is_none() =>
            {
                if let Some(v) = stream.video_data() {
                    video = Some((v.width, v.height, stream.format.clone()));
                }
            }
            _ => {}
        }
    }

    let (width, height, codec) = video.ok_or("No video stream found")?;
    Ok((
        width,
        height,
        duration.unwrap_or(0.0),
        format.unwrap_or_else(|| "unknown".to_string()),
        codec,
    ))
}

/// `Input #0, mov,mp4,m4a,3gp,3g2,mj2, from '/path/a.mp4':` -> `mov,mp4,m4a,3gp,3g2,mj2`
fn parse_input_format(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("[info]").unwrap_or(raw).trim();
    let rest = rest.strip_prefix("Input #")?;
    let (_, rest) = rest.split_once(", ")?;
    let (format, _) = rest.split_once(", from ")?;
    Some(format.to_string())
}

#[tauri::command]
pub async fn get_video_info(path: String) -> Result<VideoInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let file_path = Path::new(&path);
        let metadata = std::fs::metadata(file_path).map_err(|e| e.to_string())?;
        let name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let (width, height, duration_secs, format, codec) = probe_video(&path)?;

        Ok(VideoInfo {
            path,
            name,
            size: metadata.len(),
            width,
            height,
            duration_secs,
            format,
            codec,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn convert_video(
    app: AppHandle,
    request: ConvertVideoRequest,
) -> Result<ConvertVideoResult, String> {
    let source = request.source_path.clone();
    let original_size = std::fs::metadata(&source).map_err(|e| e.to_string())?.len();

    // Get duration for progress calculation
    let (_, _, duration_secs, _, _) = probe_video(&source)?;

    let ext = format_extension(&request.output_format);
    let stem = Path::new(&source)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output_path: PathBuf = Path::new(&request.output_dir).join(format!("{}.{}", stem, ext));
    let output_path_str = output_path.to_string_lossy().to_string();

    let codec = get_video_codec_for_format(&request.output_format);
    let quality = request.quality.unwrap_or(23); // CRF value: 0=lossless, 51=worst

    // Build ffmpeg args
    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-i".to_string(),
        source,
        "-c:v".to_string(),
        codec.to_string(),
    ];

    // Quality: map 0-100 slider to CRF (0=best=crf0, 100=worst=crf51 → invert)
    // User sees 100=best quality, 0=worst. CRF: 0=best, 51=worst.
    let crf = ((100 - quality.min(100)) as f32 * 51.0 / 100.0) as u8;
    if codec.starts_with("libvpx") {
        args.extend(["-crf".to_string(), crf.to_string(), "-b:v".to_string(), "0".to_string()]);
    } else {
        args.extend(["-crf".to_string(), crf.to_string()]);
    }

    // Resize if requested. ffmpeg's `-2` means "auto, divisible by 2"
    // so a single-axis input keeps aspect ratio while staying h264-safe.
    let scale_filter = match (request.resize_width, request.resize_height) {
        (Some(w), Some(h)) => Some(format!("scale={}:{}", w - (w % 2), h - (h % 2))),
        (Some(w), None) => Some(format!("scale={}:-2", w - (w % 2))),
        (None, Some(h)) => Some(format!("scale=-2:{}", h - (h % 2))),
        (None, None) => None,
    };
    if let Some(vf) = scale_filter {
        args.extend(["-vf".to_string(), vf]);
    }

    // Audio copy
    args.extend(["-c:a".to_string(), "aac".to_string()]);
    args.push(output_path_str.clone());

    // Run ffmpeg with progress
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let iter = FfmpegCommand::new()
            .args(args)
            .spawn()
            .map_err(|e| format!("ffmpeg spawn failed: {}", e))?
            .iter()
            .map_err(|e| format!("ffmpeg iter failed: {}", e))?;

        for event in iter {
            match event {
                FfmpegEvent::Progress(progress) => {
                    let percent = if duration_secs > 0.0 {
                        let time_secs = parse_ffmpeg_time(&progress.time);
                        ((time_secs / duration_secs) * 100.0).min(100.0)
                    } else {
                        0.0
                    };
                    let _ = app_clone.emit("video-progress", VideoProgress {
                        percent,
                        fps: progress.fps,
                        speed: format!("{:.1}x", progress.speed),
                    });
                }
                FfmpegEvent::Error(e) => {
                    // ffmpeg outputs warnings to stderr too, only fail on actual errors
                    if e.contains("Error") || e.contains("Invalid") {
                        return Err(format!("ffmpeg error: {}", e));
                    }
                }
                _ => {}
            }
        }

        let output_size = std::fs::metadata(&output_path)
            .map_err(|e| e.to_string())?
            .len();

        Ok(ConvertVideoResult {
            output_path: output_path_str,
            original_size,
            output_size,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Download ffmpeg binary if not present (development convenience)
#[tauri::command]
pub async fn ensure_ffmpeg() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| {
        match ffmpeg_sidecar::download::auto_download() {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("ffmpeg download failed: {}", e)),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_input_format_extracts_format_from_plain_line() {
        let raw = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from '/x/a.mp4':";
        assert_eq!(
            parse_input_format(raw),
            Some("mov,mp4,m4a,3gp,3g2,mj2".to_string())
        );
    }

    #[test]
    fn parse_input_format_strips_log_level_prefix() {
        let raw = "[info] Input #0, mov,mp4,m4a,3gp,3g2,mj2, from '/x/a.mp4':";
        assert_eq!(
            parse_input_format(raw),
            Some("mov,mp4,m4a,3gp,3g2,mj2".to_string())
        );
    }

    #[test]
    fn parse_input_format_returns_none_for_unrelated_line() {
        let raw = "  Duration: 00:00:23.06, start: 0.000000, bitrate: 5400 kb/s";
        assert_eq!(parse_input_format(raw), None);
    }

    /// Removes the generated fixture even if an assertion panics mid-test.
    struct TempFileGuard(PathBuf);

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn probe_video_reads_metadata_from_ffmpeg_only() {
        let path = std::env::temp_dir().join(format!("probe_video_test_{}.mp4", std::process::id()));
        let guard = TempFileGuard(path.clone());
        let path_str = path.to_string_lossy().to_string();

        let iter = FfmpegCommand::new()
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=64x48:rate=10:duration=1",
                "-pix_fmt",
                "yuv420p",
            ])
            .output(&path_str)
            .spawn()
            .expect("ffmpeg spawn failed")
            .iter()
            .expect("ffmpeg iter failed");

        for _event in iter {}

        let (width, height, duration_secs, format, codec) =
            probe_video(&path_str).expect("probe_video failed");

        drop(guard);

        assert_eq!(width, 64);
        assert_eq!(height, 48);
        assert_eq!(codec, "h264");
        assert!((0.8..=1.2).contains(&duration_secs));
        assert!(format.contains("mp4"));
    }
}
