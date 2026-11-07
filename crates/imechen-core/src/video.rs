use crate::cancel::{CancelToken, JobGuard, Outcome};
use crate::output::OutputReservation;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress, LogLevel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

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

/// The WebM muxer accepts only Vorbis or Opus audio, so AAC there makes ffmpeg
/// fail on any source that has an audio track.
fn get_audio_codec_for_format(format: &str) -> &str {
    match format.to_lowercase().as_str() {
        "webm" => "libopus",
        _ => "aac",
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

/// Fails on ffmpeg's exit status rather than on its log lines: a muxer that
/// rejects a codec logs the reason at error level and still leaves a
/// header-only file behind, which looks like a small but valid output.
pub(crate) fn run_ffmpeg<I, S>(
    args: I,
    cancel: &CancelToken,
    mut on_progress: impl FnMut(&FfmpegProgress),
) -> Result<Outcome<()>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut child = FfmpegCommand::new()
        .args(args)
        .spawn()
        .map_err(|e| format!("ffmpeg spawn failed: {}", e))?;
    let events = match child.iter() {
        Ok(events) => events,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("ffmpeg iter failed: {}", e));
        }
    };
    let child = Arc::new(Mutex::new(child));
    // Attaching makes CancelToken::cancel() kill ffmpeg directly (a graceful
    // quit would finalize a file that is discarded anyway); it also kills
    // right away if cancel already ran.
    cancel.attach(child.clone());

    // The first error line names the cause; later ones are its fallout.
    // Killing closes ffmpeg's stderr, which ends this loop by itself.
    let mut first_error: Option<String> = None;
    for event in events {
        match event {
            FfmpegEvent::Progress(progress) => on_progress(&progress),
            FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, line)
            | FfmpegEvent::Error(line) => {
                first_error.get_or_insert(line);
            }
            _ => {}
        }
    }

    // wait() holds the child lock until ffmpeg exits; detaching first keeps a
    // concurrent cancel() from blocking on that lock.
    cancel.detach();
    // Also required after kill: on Windows the output file stays locked until
    // the process has exited, and OutputReservation::drop ignores the remove
    // error, so returning early would leave the partial file behind.
    let status = child
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .wait()
        .map_err(|e| format!("ffmpeg wait failed: {}", e))?;
    if cancel.is_cancelled() {
        Ok(Outcome::Cancelled)
    } else if status.success() {
        Ok(Outcome::Done(()))
    } else {
        Err(format!(
            "ffmpeg error: {}",
            first_error.unwrap_or_else(|| status.to_string())
        ))
    }
}

/// `Input #0, mov,mp4,m4a,3gp,3g2,mj2, from '/path/a.mp4':` -> `mov,mp4,m4a,3gp,3g2,mj2`
fn parse_input_format(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("[info]").unwrap_or(raw).trim();
    let rest = rest.strip_prefix("Input #")?;
    let (_, rest) = rest.split_once(", ")?;
    let (format, _) = rest.split_once(", from ")?;
    Some(format.to_string())
}

pub fn get_video_info(path: &str) -> Result<VideoInfo, String> {
    let file_path = Path::new(path);
    let metadata = std::fs::metadata(file_path).map_err(|e| e.to_string())?;
    let name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (width, height, duration_secs, format, codec) = probe_video(path)?;

    Ok(VideoInfo {
        path: path.to_string(),
        name,
        size: metadata.len(),
        width,
        height,
        duration_secs,
        format,
        codec,
    })
}

pub fn convert_video(
    request: ConvertVideoRequest,
    cancel: &CancelToken,
    mut on_progress: impl FnMut(VideoProgress),
) -> Result<Outcome<ConvertVideoResult>, String> {
    // Declared first so it drops last, after the reservation removed any partial file.
    let _job = JobGuard::new();
    if cancel.is_cancelled() {
        return Ok(Outcome::Cancelled);
    }
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
    let reservation = OutputReservation::reserve(Path::new(&request.output_dir), &stem, ext)?;
    let output_path = reservation.path().to_path_buf();
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
        args.extend([
            "-crf".to_string(),
            crf.to_string(),
            "-b:v".to_string(),
            "0".to_string(),
        ]);
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

    args.extend([
        "-c:a".to_string(),
        get_audio_codec_for_format(&request.output_format).to_string(),
    ]);
    args.push(output_path_str.clone());

    let finished = run_ffmpeg(args, cancel, |progress| {
        let percent = if duration_secs > 0.0 {
            let time_secs = parse_ffmpeg_time(&progress.time);
            ((time_secs / duration_secs) * 100.0).min(100.0)
        } else {
            0.0
        };
        on_progress(VideoProgress {
            percent,
            fps: progress.fps,
            speed: format!("{:.1}x", progress.speed),
        });
    })?;
    if finished == Outcome::Cancelled {
        return Ok(Outcome::Cancelled);
    }

    let output_size = std::fs::metadata(&output_path)
        .map_err(|e| e.to_string())?
        .len();
    // The reservation is an empty file, so ffmpeg exiting without writing
    // anything would otherwise look like a successful empty conversion.
    if output_size == 0 {
        return Err("ffmpeg produced no output".to_string());
    }
    reservation.commit();

    Ok(Outcome::Done(ConvertVideoResult {
        output_path: output_path_str,
        original_size,
        output_size,
    }))
}

/// ffmpeg-sidecar's `auto_download` fetches floating "latest" URLs from
/// third-party hosts and installs whatever comes back, so each release-CI
/// target is pinned to an exact archive and digest instead. Dependabot cannot
/// bump these; update the URL and digest together by hand.
fn pinned_ffmpeg_archive() -> Option<(&'static str, &'static str)> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some((
            "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip",
            "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
        ))
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        // martin-riedl stops publishing Intel macOS builds in 2027-01; existing
        // build URLs are expected to stay, but no newer pin will be available.
        Some((
            "https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip",
            "7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4",
        ))
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some((
            "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip",
            "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba",
        ))
    }
    // BtbN has no permanent per-version tags; only month-end autobuilds are kept,
    // for two years. These expire around 2028-09, after which the download
    // fails the digest/HTTP check instead of installing something unverified.
    else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some((
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-n9.0.2-17-g2a571b6068-linux64-gpl-9.0.tar.xz",
            "68ee646831adaae2495618346f3bba94ff207ff83bbd34d643e7004730d66269",
        ))
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some((
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-n9.0.2-17-g2a571b6068-linuxarm64-gpl-9.0.tar.xz",
            "91afcd7695d4bbbb9e0057aa10baaf9875ec4e1a97303f9059ec971336a2ab46",
        ))
    } else {
        None
    }
}

/// Must run before unpacking: extraction itself parses attacker-controlled
/// archive metadata, so an unverified archive is never handed to the unpacker.
fn verify_sha256(path: &Path, expected_hex: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    let actual = format!("{:x}", hasher.finalize());

    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        let _ = std::fs::remove_file(path);
        Err(format!(
            "ffmpeg archive checksum mismatch (expected {}, got {})",
            expected_hex, actual
        ))
    }
}

/// Download ffmpeg binary if not present (development convenience)
pub fn ensure_ffmpeg() -> Result<bool, String> {
    use ffmpeg_sidecar::command::ffmpeg_is_installed;
    use ffmpeg_sidecar::download::{download_ffmpeg_package, unpack_ffmpeg_without_extras};
    use ffmpeg_sidecar::paths::sidecar_dir;

    if ffmpeg_is_installed() {
        return Ok(true);
    }

    let (url, sha256) = pinned_ffmpeg_archive()
        .ok_or("No verified ffmpeg build for this platform; install ffmpeg manually")?;
    let dir = sidecar_dir().map_err(|e| format!("ffmpeg download failed: {}", e))?;
    let archive =
        download_ffmpeg_package(url, &dir).map_err(|e| format!("ffmpeg download failed: {}", e))?;
    verify_sha256(&archive, sha256)?;
    // probe_video and convert_video only need ffmpeg itself.
    unpack_ffmpeg_without_extras(&archive, &dir)
        .map_err(|e| format!("ffmpeg unpack failed: {}", e))?;

    if ffmpeg_is_installed() {
        Ok(true)
    } else {
        Err("ffmpeg failed to install, please install manually".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn write_fixture(tag: &str, bytes: &[u8]) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("verify_sha256_{}_{}", tag, std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn verify_sha256_accepts_matching_digest_in_any_case() {
        let path = write_fixture("match", b"abc");
        let _guard = TempFileGuard(path.clone());

        assert_eq!(verify_sha256(&path, ABC_SHA256), Ok(()));
        assert_eq!(verify_sha256(&path, &ABC_SHA256.to_uppercase()), Ok(()));
        assert!(path.exists());
    }

    #[test]
    fn verify_sha256_rejects_and_deletes_tampered_archive() {
        let path = write_fixture("mismatch", b"abd");
        let _guard = TempFileGuard(path.clone());

        let err = verify_sha256(&path, ABC_SHA256).unwrap_err();

        assert!(err.contains("checksum mismatch"), "{}", err);
        assert!(!path.exists());
    }

    #[test]
    fn pinned_ffmpeg_archive_digests_are_sha256_hex() {
        if let Some((url, sha256)) = pinned_ffmpeg_archive() {
            assert!(url.starts_with("https://"));
            assert_eq!(sha256.len(), 64);
            assert!(sha256.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    /// Opt-in because it needs the real pinned archive for this host:
    /// `IMECHEN_FFMPEG_ARCHIVE=/path/to/archive cargo test -- --ignored`
    #[test]
    #[ignore]
    fn pinned_archive_verifies_unpacks_and_runs() {
        use ffmpeg_sidecar::download::unpack_ffmpeg_without_extras;

        let Ok(src) = std::env::var("IMECHEN_FFMPEG_ARCHIVE") else {
            return;
        };
        let (url, sha256) = pinned_ffmpeg_archive().expect("no pin for this host");
        let dir = std::env::temp_dir().join(format!("imechen_unpack_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = TempDirGuard(dir.clone());
        let archive = dir.join(Path::new(url).file_name().unwrap());
        std::fs::copy(&src, &archive).unwrap();

        verify_sha256(&archive, sha256).expect("digest mismatch");
        unpack_ffmpeg_without_extras(&archive, &dir).expect("unpack failed");

        let bin = dir.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        let status = std::process::Command::new(&bin)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .status()
            .expect("extracted ffmpeg is not executable");
        assert!(status.success());
    }

    struct TempDirGuard(PathBuf);

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
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
        let path =
            std::env::temp_dir().join(format!("probe_video_test_{}.mp4", std::process::id()));
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

    fn generate_clip(tag: &str, secs: u32) -> (String, TempDirGuard, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("imechen_video_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let guard = TempDirGuard(dir.clone());
        let clip = dir.join("clip.mp4").to_string_lossy().to_string();

        // The audio track matters: some containers reject the default audio
        // codec, which a silent clip would never exercise.
        let iter = FfmpegCommand::new()
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("testsrc=size=64x48:rate=10:duration={}", secs),
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency=440:duration={}", secs),
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .output(&clip)
            .spawn()
            .expect("ffmpeg spawn failed")
            .iter()
            .expect("ffmpeg iter failed");
        for _event in iter {}

        (clip, guard, dir)
    }

    fn done<T>(result: Result<Outcome<T>, String>) -> Result<T, String> {
        match result? {
            Outcome::Done(v) => Ok(v),
            Outcome::Cancelled => Err("unexpectedly cancelled".to_string()),
        }
    }

    #[test]
    fn convert_video_writes_output_and_reports_progress() {
        for format in ["webm", "mp4"] {
            let (clip, _guard, dir) = generate_clip(format, 3);
            let out_dir = dir.join("out");
            std::fs::create_dir_all(&out_dir).unwrap();
            let request = ConvertVideoRequest {
                source_path: clip.clone(),
                output_format: format.to_string(),
                output_dir: out_dir.to_string_lossy().to_string(),
                quality: None,
                resize_width: None,
                resize_height: None,
            };
            let mut progress = Vec::new();

            let result = done(convert_video(request, &CancelToken::new(), |p| {
                progress.push(p)
            }))
            .unwrap_or_else(|e| panic!("convert_video to {} failed: {}", format, e));

            assert_eq!(
                Path::new(&result.output_path),
                out_dir.join(format!("clip.{}", format))
            );
            assert!(result.output_size > 0, "{}", format);
            assert_eq!(
                result.original_size,
                std::fs::metadata(&clip).unwrap().len()
            );
            assert!(!progress.is_empty(), "no progress reported for {}", format);
            assert!(progress.iter().all(|p| (0.0..=100.0).contains(&p.percent)));
        }
    }

    #[test]
    fn convert_video_in_the_source_directory_keeps_the_source_intact() {
        let (clip, _guard, dir) = generate_clip("inplace", 1);
        let before = std::fs::read(&clip).unwrap();
        let request = ConvertVideoRequest {
            source_path: clip.clone(),
            output_format: "mp4".to_string(),
            output_dir: dir.to_string_lossy().to_string(),
            quality: None,
            resize_width: None,
            resize_height: None,
        };

        let result = done(convert_video(request, &CancelToken::new(), |_| {}))
            .expect("convert_video failed");

        assert_eq!(Path::new(&result.output_path), dir.join("clip(1).mp4"));
        assert_eq!(std::fs::read(&clip).unwrap(), before);
        assert!(result.output_size > 0);
    }

    #[test]
    fn failed_video_conversion_leaves_no_reserved_file() {
        let (clip, _guard, dir) = generate_clip("failed", 1);
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        // h264 caps frame size, so ffmpeg fails after the name is reserved.
        let request = ConvertVideoRequest {
            source_path: clip,
            output_format: "mp4".to_string(),
            output_dir: out_dir.to_string_lossy().to_string(),
            quality: None,
            resize_width: Some(100_000),
            resize_height: Some(100_000),
        };

        assert!(convert_video(request, &CancelToken::new(), |_| {}).is_err());

        assert_eq!(std::fs::read_dir(&out_dir).unwrap().count(), 0);
    }

    fn mp4_request(clip: &str, out_dir: &Path) -> ConvertVideoRequest {
        ConvertVideoRequest {
            source_path: clip.to_string(),
            output_format: "mp4".to_string(),
            output_dir: out_dir.to_string_lossy().to_string(),
            quality: Some(100),
            resize_width: None,
            resize_height: None,
        }
    }

    #[test]
    fn pre_cancelled_video_conversion_is_cancelled_and_writes_nothing() {
        let (clip, _guard, dir) = generate_clip("precancel", 1);
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        let cancel = CancelToken::new();
        cancel.cancel();

        let outcome = convert_video(mp4_request(&clip, &out_dir), &cancel, |_| {});

        assert!(matches!(outcome, Ok(Outcome::Cancelled)));
        assert_eq!(std::fs::read_dir(&out_dir).unwrap().count(), 0);
    }

    #[test]
    fn video_conversion_cancelled_mid_run_stops_early_and_leaves_no_output() {
        let (clip, _guard, dir) = generate_clip("midcancel", 30);
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        // Upscaling keeps the encode busy long enough to tell a stop from a finish.
        let big = |dir: &Path| {
            let mut request = mp4_request(&clip, dir);
            request.resize_width = Some(3840);
            request.resize_height = Some(2160);
            request
        };
        let full_dir = dir.join("full");
        std::fs::create_dir_all(&full_dir).unwrap();
        let started = std::time::Instant::now();
        let full = convert_video(big(&full_dir), &CancelToken::new(), |_| {});
        let full_elapsed = started.elapsed();
        assert!(matches!(full, Ok(Outcome::Done(_))));

        let cancel = CancelToken::new();
        let trigger = cancel.clone();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            trigger.cancel();
        });
        let started = std::time::Instant::now();

        let outcome = convert_video(big(&out_dir), &cancel, |_| {
            assert!(crate::cancel::active_jobs() >= 1);
        });
        let elapsed = started.elapsed();
        canceller.join().unwrap();
        eprintln!(
            "full encode {:?}, cancelled run {:?}",
            full_elapsed, elapsed
        );

        assert!(matches!(outcome, Ok(Outcome::Cancelled)));
        assert!(
            elapsed < full_elapsed / 2,
            "{:?} vs {:?}",
            elapsed,
            full_elapsed
        );
        assert_eq!(std::fs::read_dir(&out_dir).unwrap().count(), 0);
    }
}
