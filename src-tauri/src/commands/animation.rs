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

/// `output_name` arrives from the webview; anything that is not a bare file
/// name would let the join below escape the user-chosen `output_dir`.
fn validate_output_name(name: &str) -> Result<&str, String> {
    let is_bare = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\'])
        && Path::new(name).file_name().is_some_and(|f| f == name);
    if is_bare {
        Ok(name)
    } else {
        Err(format!("Invalid output name: {}", name))
    }
}

/// Removes the per-run frame directory on every exit path, including `?` early returns.
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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

    if image_paths.is_empty() {
        return Err("No images provided".to_string());
    }

    // A fixed name in the shared temp dir could be pre-created (or symlinked) by
    // another user, and stale frames from a crashed run would leak into the
    // sequence. `create_dir` fails instead of reusing an existing path.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp_dir = std::env::temp_dir().join(format!(
        "imechen_frames_{}_{}",
        std::process::id(),
        nanos
    ));
    std::fs::create_dir(&tmp_dir).map_err(|e| e.to_string())?;
    let _guard = TempDirGuard(tmp_dir.clone());

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
                return Err(format!("ffmpeg error: {}", e));
            }
        }
    }

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
        if request.fps == 0 {
            return Err("fps must be at least 1".to_string());
        }
        let output_name = validate_output_name(&request.output_name)?;

        let output_path: PathBuf =
            Path::new(&request.output_dir).join(format!("{}.{}", output_name, ext));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_output_name_accepts_bare_file_name() {
        assert_eq!(validate_output_name("animation"), Ok("animation"));
        assert_eq!(validate_output_name("my anim.v2"), Ok("my anim.v2"));
    }

    #[test]
    fn validate_output_name_rejects_names_that_escape_output_dir() {
        for name in ["", ".", "..", "../evil", "a/b", "/etc/passwd", "a\\b", "..\\evil"] {
            assert!(validate_output_name(name).is_err(), "accepted {:?}", name);
        }
    }

    fn frame_dirs() -> Vec<PathBuf> {
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("imechen_frames_"))
            })
            .collect()
    }

    #[test]
    fn create_mp4_from_frames_encodes_and_removes_its_temp_dir() {
        let work = std::env::temp_dir().join(format!("imechen_anim_test_{}", std::process::id()));
        std::fs::create_dir_all(&work).unwrap();
        let _work_guard = TempDirGuard(work.clone());

        let paths: Vec<String> = (0..3u8)
            .map(|i| {
                let p = work.join(format!("f{}.png", i));
                RgbaImage::from_pixel(16, 16, image::Rgba([i * 80, 0, 0, 255]))
                    .save(&p)
                    .unwrap();
                p.to_string_lossy().to_string()
            })
            .collect();
        let before = frame_dirs();
        let out = work.join("out.mp4");

        create_mp4_from_frames(&paths, &out, 10).expect("create_mp4_from_frames failed");

        assert!(std::fs::metadata(&out).unwrap().len() > 0);
        let leaked: Vec<_> = frame_dirs().into_iter().filter(|d| !before.contains(d)).collect();
        assert!(leaked.is_empty(), "temp frame dirs left behind: {:?}", leaked);
    }
}
