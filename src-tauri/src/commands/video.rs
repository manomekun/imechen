use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
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

/// Get ffprobe path (next to ffmpeg binary)
fn ffprobe_path() -> PathBuf {
    let ffmpeg = ffmpeg_sidecar::paths::ffmpeg_path();
    let dir = ffmpeg.parent().unwrap_or(Path::new("."));
    dir.join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" })
}

/// Parse video metadata using ffprobe (bundled with ffmpeg-sidecar)
fn probe_video(path: &str) -> Result<(u32, u32, f64, String, String), String> {
    let output = Command::new(ffprobe_path())
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            path,
        ])
        .output()
        .map_err(|e| format!("ffprobe failed: {}", e))?;

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("ffprobe parse error: {}", e))?;

    let video_stream = json["streams"]
        .as_array()
        .and_then(|streams| {
            streams.iter().find(|s| s["codec_type"].as_str() == Some("video"))
        })
        .ok_or("No video stream found")?;

    let width = video_stream["width"].as_u64().unwrap_or(0) as u32;
    let height = video_stream["height"].as_u64().unwrap_or(0) as u32;
    let codec = video_stream["codec_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let duration = json["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let format = json["format"]["format_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    Ok((width, height, duration, format, codec))
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

    // Resize if requested
    if let (Some(w), Some(h)) = (request.resize_width, request.resize_height) {
        // Ensure even dimensions for h264
        let w = w - (w % 2);
        let h = h - (h % 2);
        args.extend(["-vf".to_string(), format!("scale={}:{}", w, h)]);
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
