use crate::cancel::{CancelToken, JobGuard, Outcome};
use crate::output::OutputReservation;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{imageops::FilterType, Frame, RgbaImage};
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

/// `output_name` arrives from the UI layer; anything that is not a bare file
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
fn load_frames(
    paths: &[String],
    cancel: &CancelToken,
) -> Result<Outcome<(Vec<RgbaImage>, u32, u32)>, String> {
    if paths.is_empty() {
        return Err("No images provided".to_string());
    }

    let first =
        image::open(&paths[0]).map_err(|e| format!("Failed to open {}: {}", paths[0], e))?;
    let (width, height) = (first.width(), first.height());
    let mut frames = vec![first.to_rgba8()];

    for path in &paths[1..] {
        if cancel.is_cancelled() {
            return Ok(Outcome::Cancelled);
        }
        let img = image::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?;
        // Resize to match first frame if dimensions differ
        let resized = if img.width() != width || img.height() != height {
            image::imageops::resize(&img.to_rgba8(), width, height, FilterType::Lanczos3)
        } else {
            img.to_rgba8()
        };
        frames.push(resized);
    }

    Ok(Outcome::Done((frames, width, height)))
}

fn create_gif(
    frames: &[RgbaImage],
    output_path: &Path,
    fps: u32,
    cancel: &CancelToken,
) -> Result<Outcome<()>, String> {
    let delay_ms = (1000.0 / fps as f64) as u32;
    let delay = image::Delay::from_numer_denom_ms(delay_ms, 1);

    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut encoder = GifEncoder::new(file);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|e| e.to_string())?;

    for rgba in frames {
        if cancel.is_cancelled() {
            return Ok(Outcome::Cancelled);
        }
        let frame = Frame::from_parts(rgba.clone(), 0, 0, delay);
        encoder.encode_frame(frame).map_err(|e| e.to_string())?;
    }

    Ok(Outcome::Done(()))
}

fn create_awebp(
    frames: &[RgbaImage],
    width: u32,
    height: u32,
    output_path: &Path,
    fps: u32,
    cancel: &CancelToken,
) -> Result<Outcome<()>, String> {
    let timestamp_ms_step = (1000.0 / fps as f64) as i32;
    let config = webp::WebPConfig::new().map_err(|_| "Failed to create WebP config")?;

    let mut encoder = webp::AnimEncoder::new(width, height, &config);

    let mut timestamp = 0i32;
    for rgba in frames {
        if cancel.is_cancelled() {
            return Ok(Outcome::Cancelled);
        }
        let frame = webp::AnimFrame::from_rgba(rgba.as_raw(), width, height, timestamp);
        encoder.add_frame(frame);
        timestamp += timestamp_ms_step;
    }

    // encode() is one-shot and cannot be interrupted.
    let webp_data = encoder.encode();
    std::fs::write(output_path, &*webp_data).map_err(|e| e.to_string())?;
    Ok(Outcome::Done(()))
}

fn create_mp4_from_frames(
    image_paths: &[String],
    output_path: &Path,
    fps: u32,
    cancel: &CancelToken,
) -> Result<Outcome<()>, String> {
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
    let tmp_dir =
        std::env::temp_dir().join(format!("imechen_frames_{}_{}", std::process::id(), nanos));
    std::fs::create_dir(&tmp_dir).map_err(|e| e.to_string())?;
    let _guard = TempDirGuard(tmp_dir.clone());

    // Create symlinks/copies with sequential names for ffmpeg
    let ext = Path::new(&image_paths[0])
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    for (i, path) in image_paths.iter().enumerate() {
        if cancel.is_cancelled() {
            return Ok(Outcome::Cancelled);
        }
        let dest = tmp_dir.join(format!("frame_{:06}.{}", i, ext));
        std::fs::copy(path, &dest).map_err(|e| format!("Failed to copy frame: {}", e))?;
    }

    let pattern = tmp_dir.join(format!("frame_%06d.{}", ext));

    crate::video::run_ffmpeg(
        [
            "-y",
            "-framerate",
            &fps.to_string(),
            "-i",
            &pattern.to_string_lossy(),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "18",
            &output_path.to_string_lossy(),
        ],
        cancel,
        |_| {},
    )
}

pub fn create_animation(
    request: CreateAnimationRequest,
    cancel: &CancelToken,
) -> Result<Outcome<CreateAnimationResult>, String> {
    // Declared first so it drops last, after the reservation removed any partial file.
    let _job = JobGuard::new();
    if cancel.is_cancelled() {
        return Ok(Outcome::Cancelled);
    }
    let paths = &request.image_paths;
    let frame_count = paths.len();

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

    let reservation = OutputReservation::reserve(Path::new(&request.output_dir), output_name, ext)?;
    let output_path = reservation.path().to_path_buf();

    let finished = match request.output_format.to_lowercase().as_str() {
        "gif" => match load_frames(paths, cancel)? {
            Outcome::Done((frames, _, _)) => {
                create_gif(&frames, &output_path, request.fps, cancel)?
            }
            Outcome::Cancelled => Outcome::Cancelled,
        },
        "awebp" | "webp" => match load_frames(paths, cancel)? {
            Outcome::Done((frames, width, height)) => {
                create_awebp(&frames, width, height, &output_path, request.fps, cancel)?
            }
            Outcome::Cancelled => Outcome::Cancelled,
        },
        "mp4" => create_mp4_from_frames(paths, &output_path, request.fps, cancel)?,
        _ => unreachable!(),
    };
    if finished == Outcome::Cancelled {
        return Ok(Outcome::Cancelled);
    }

    let output_size = std::fs::metadata(&output_path)
        .map_err(|e| e.to_string())?
        .len();

    let output_path = reservation.commit();

    Ok(Outcome::Done(CreateAnimationResult {
        output_path: output_path.to_string_lossy().to_string(),
        frame_count,
        output_size,
    }))
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
        for name in [
            "",
            ".",
            "..",
            "../evil",
            "a/b",
            "/etc/passwd",
            "a\\b",
            "..\\evil",
        ] {
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

        create_mp4_from_frames(&paths, &out, 10, &CancelToken::new())
            .expect("create_mp4_from_frames failed");

        assert!(std::fs::metadata(&out).unwrap().len() > 0);
        let leaked: Vec<_> = frame_dirs()
            .into_iter()
            .filter(|d| !before.contains(d))
            .collect();
        assert!(
            leaked.is_empty(),
            "temp frame dirs left behind: {:?}",
            leaked
        );
    }

    fn write_frames(dir: &Path, count: u8) -> Vec<String> {
        (0..count)
            .map(|i| {
                let p = dir.join(format!("g{}.png", i));
                RgbaImage::from_pixel(16, 16, image::Rgba([i * 60, 100, 0, 255]))
                    .save(&p)
                    .unwrap();
                p.to_string_lossy().to_string()
            })
            .collect()
    }

    fn request(dir: &Path, paths: Vec<String>, format: &str, fps: u32) -> CreateAnimationRequest {
        CreateAnimationRequest {
            image_paths: paths,
            output_format: format.to_string(),
            output_dir: dir.to_string_lossy().to_string(),
            output_name: "anim".to_string(),
            fps,
        }
    }

    fn create_done(req: CreateAnimationRequest) -> Result<CreateAnimationResult, String> {
        match create_animation(req, &CancelToken::new())? {
            Outcome::Done(v) => Ok(v),
            Outcome::Cancelled => Err("unexpectedly cancelled".to_string()),
        }
    }

    fn work_dir(tag: &str) -> (PathBuf, TempDirGuard) {
        let dir = std::env::temp_dir().join(format!("imechen_anim_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (dir.clone(), TempDirGuard(dir))
    }

    #[test]
    fn gif_and_animated_webp_are_written_with_the_requested_frame_count() {
        for (format, ext) in [("gif", "gif"), ("awebp", "webp")] {
            let (dir, _guard) = work_dir(format);
            let paths = write_frames(&dir, 3);

            let result =
                create_done(request(&dir, paths, format, 10)).expect("create_animation failed");

            assert_eq!(result.frame_count, 3, "{}", format);
            assert_eq!(
                Path::new(&result.output_path),
                dir.join(format!("anim.{}", ext))
            );
            assert!(result.output_size > 0);
            assert_eq!(
                result.output_size,
                std::fs::metadata(&result.output_path).unwrap().len()
            );
        }
    }

    #[test]
    fn zero_fps_is_rejected() {
        let (dir, _guard) = work_dir("fps0");
        let paths = write_frames(&dir, 2);

        let err = create_done(request(&dir, paths, "gif", 0)).unwrap_err();

        assert!(err.contains("fps"), "{}", err);
    }

    #[test]
    fn unsupported_animation_format_is_rejected() {
        let (dir, _guard) = work_dir("badfmt");
        let paths = write_frames(&dir, 2);

        let err = create_done(request(&dir, paths, "apng", 10)).unwrap_err();

        assert!(err.contains("Unsupported format"), "{}", err);
    }

    #[test]
    fn existing_animation_is_not_overwritten() {
        let (dir, _guard) = work_dir("exists");
        let paths = write_frames(&dir, 2);
        std::fs::write(dir.join("anim.gif"), b"keep me").unwrap();

        let result = create_done(request(&dir, paths, "gif", 10)).expect("create_animation failed");

        assert_eq!(Path::new(&result.output_path), dir.join("anim(1).gif"));
        assert_eq!(std::fs::read(dir.join("anim.gif")).unwrap(), b"keep me");
        assert!(result.output_size > 0);
    }

    #[test]
    fn failed_animation_leaves_no_reserved_file() {
        let (dir, _guard) = work_dir("failed");
        let missing = vec![dir.join("missing.png").to_string_lossy().to_string()];

        assert!(create_done(request(&dir, missing, "gif", 10)).is_err());

        assert!(!dir.join("anim.gif").exists());
    }

    #[test]
    fn pre_cancelled_animation_is_cancelled_and_writes_nothing() {
        for format in ["gif", "awebp", "mp4"] {
            let (dir, _guard) = work_dir(&format!("precancel_{}", format));
            let paths = write_frames(&dir, 3);
            let before = std::fs::read_dir(&dir).unwrap().count();
            let cancel = CancelToken::new();
            cancel.cancel();

            let outcome = create_animation(request(&dir, paths, format, 10), &cancel);

            assert!(matches!(outcome, Ok(Outcome::Cancelled)), "{}", format);
            assert_eq!(
                std::fs::read_dir(&dir).unwrap().count(),
                before,
                "{}",
                format
            );
        }
    }

    #[test]
    fn cancelled_frame_loading_reports_cancelled() {
        let (dir, _guard) = work_dir("load_cancel");
        let paths = write_frames(&dir, 3);
        let cancel = CancelToken::new();
        cancel.cancel();

        let outcome = load_frames(&paths, &cancel);

        assert!(matches!(outcome, Ok(Outcome::Cancelled)));
    }

    #[test]
    fn cancelled_gif_encoding_reports_cancelled() {
        let (dir, _guard) = work_dir("gif_cancel");
        let frames = vec![RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255])); 3];
        let cancel = CancelToken::new();
        cancel.cancel();

        let outcome = create_gif(&frames, &dir.join("a.gif"), 10, &cancel);

        assert!(matches!(outcome, Ok(Outcome::Cancelled)));
    }
}
