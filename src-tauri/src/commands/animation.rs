use image::codecs::gif::{GifEncoder, Repeat};
use image::{Frame, RgbaImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct CreateAnimationRequest {
    pub image_paths: Vec<String>,
    pub output_format: String,
    pub output_dir: String,
    pub output_name: String,
    pub fps: u32,
}

#[derive(Debug, Serialize)]
pub struct CreateAnimationResult {
    pub output_path: String,
    pub frame_count: usize,
    pub output_size: u64,
}

/// Load and optionally resize images to match the first frame's dimensions
fn load_frames(paths: &[String]) -> Result<(Vec<RgbaImage>, u32, u32), String> {
    if paths.is_empty() {
        return Err("No images provided".to_string());
    }

    let first = image::open(&paths[0]).map_err(|e| format!("Failed to open {}: {}", paths[0], e))?;
    let (width, height) = (first.width(), first.height());
    let mut frames = vec![first.to_rgba8()];

    for path in &paths[1..] {
        let img = image::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?;
        // Resize to match first frame if dimensions differ
        let resized = if img.width() != width || img.height() != height {
            image::imageops::resize(&img.to_rgba8(), width, height, FilterType::Lanczos3)
        } else {
            img.to_rgba8()
        };
        frames.push(resized);
    }

    Ok((frames, width, height))
}

fn create_gif(
    frames: &[RgbaImage],
    output_path: &Path,
    fps: u32,
) -> Result<(), String> {
    let delay_ms = (1000.0 / fps as f64) as u32;
    let delay = image::Delay::from_numer_denom_ms(delay_ms, 1);

    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;

    for rgba in frames {
        let frame = Frame::from_parts(rgba.clone(), 0, 0, delay);
        encoder.encode_frame(frame).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn create_awebp(
    frames: &[RgbaImage],
    width: u32,
    height: u32,
    output_path: &Path,
    fps: u32,
) -> Result<(), String> {
    let timestamp_ms_step = (1000.0 / fps as f64) as i32;
    let config = webp::WebPConfig::new().map_err(|_| "Failed to create WebP config")?;

    let mut encoder = webp::AnimEncoder::new(width, height, &config);

    let mut timestamp = 0i32;
    for rgba in frames {
        let frame = webp::AnimFrame::from_rgba(rgba.as_raw(), width, height, timestamp);
        encoder.add_frame(frame);
        timestamp += timestamp_ms_step;
    }

    let webp_data = encoder.encode();
    std::fs::write(output_path, &*webp_data).map_err(|e| e.to_string())?;
    Ok(())
}

fn create_mp4_from_frames(
    image_paths: &[String],
    output_path: &Path,
    fps: u32,
) -> Result<(), String> {
    use ffmpeg_sidecar::command::FfmpegCommand;

    // Create a temporary file list for ffmpeg concat
    let tmp_dir = std::env::temp_dir().join("imechen_frames");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

    // Create symlinks/copies with sequential names for ffmpeg
    let ext = Path::new(&image_paths[0])
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    for (i, path) in image_paths.iter().enumerate() {
        let dest = tmp_dir.join(format!("frame_{:06}.{}", i, ext));
        std::fs::copy(path, &dest).map_err(|e| format!("Failed to copy frame: {}", e))?;
    }

    let pattern = tmp_dir.join(format!("frame_%06d.{}", ext));

    let iter = FfmpegCommand::new()
        .args([
            "-y",
            "-framerate", &fps.to_string(),
            "-i", &pattern.to_string_lossy(),
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-crf", "18",
            &output_path.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| format!("ffmpeg spawn failed: {}", e))?
        .iter()
        .map_err(|e| format!("ffmpeg iter failed: {}", e))?;

    for event in iter {
        if let ffmpeg_sidecar::event::FfmpegEvent::Error(e) = event {
            if e.contains("Error") || e.contains("Invalid") {
                // Cleanup temp
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Err(format!("ffmpeg error: {}", e));
            }
        }
    }

    // Cleanup temp
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(())
}

#[tauri::command]
pub async fn create_animation(
    request: CreateAnimationRequest,
) -> Result<CreateAnimationResult, String> {
    let paths = request.image_paths.clone();
    let frame_count = paths.len();

    tauri::async_runtime::spawn_blocking(move || {
        let ext = match request.output_format.to_lowercase().as_str() {
            "gif" => "gif",
            "awebp" | "webp" => "webp",
            "mp4" => "mp4",
            _ => return Err(format!("Unsupported format: {}", request.output_format)),
        };

        let output_path: PathBuf =
            Path::new(&request.output_dir).join(format!("{}.{}", request.output_name, ext));

        match request.output_format.to_lowercase().as_str() {
            "gif" => {
                let (frames, _, _) = load_frames(&paths)?;
                create_gif(&frames, &output_path, request.fps)?;
            }
            "awebp" | "webp" => {
                let (frames, width, height) = load_frames(&paths)?;
                create_awebp(&frames, width, height, &output_path, request.fps)?;
            }
            "mp4" => {
                create_mp4_from_frames(&paths, &output_path, request.fps)?;
            }
            _ => unreachable!(),
        }

        let output_size = std::fs::metadata(&output_path)
            .map_err(|e| e.to_string())?
            .len();

        Ok(CreateAnimationResult {
            output_path: output_path.to_string_lossy().to_string(),
            frame_count,
            output_size,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
