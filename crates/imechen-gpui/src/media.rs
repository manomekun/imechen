use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp"];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "avi", "webm", "flv", "mkv"];

pub fn classify(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        Some(MediaKind::Video)
    } else {
        None
    }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = "B";
    for next in UNITS {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    format!("{value:.1} {unit}")
}

pub fn format_duration(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let (h, m, s) = (total / 3600, total % 3600 / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_supported_image_extensions_case_insensitively() {
        for name in ["a.jpg", "a.JPEG", "a.png", "a.gif", "a.WebP", "a.bmp"] {
            assert_eq!(classify(Path::new(name)), Some(MediaKind::Image), "{name}");
        }
    }

    #[test]
    fn classifies_supported_video_extensions() {
        for name in ["a.mp4", "a.MOV", "a.avi", "a.webm", "a.flv", "a.mkv"] {
            assert_eq!(classify(Path::new(name)), Some(MediaKind::Video), "{name}");
        }
    }

    #[test]
    fn rejects_unknown_or_missing_extensions() {
        assert_eq!(classify(Path::new("a.txt")), None);
        assert_eq!(classify(Path::new("noext")), None);
    }

    #[test]
    fn formats_sizes_with_binary_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn formats_durations_with_hours_only_when_needed() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(65.4), "1:05");
        assert_eq!(format_duration(3725.0), "1:02:05");
    }
}
