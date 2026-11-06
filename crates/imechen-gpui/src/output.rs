use std::path::{Path, PathBuf};

/// Mirrors how core names its output (`<stem>.<ext>` inside `dir`). Core does
/// not expose the path it will write, and the same-path guard must know it
/// before converting.
pub fn target_path(source: &Path, dir: &Path, extension: &str) -> PathBuf {
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    dir.join(format!("{stem}.{extension}"))
}

/// Core overwrites its output without asking, so converting onto the source
/// would destroy it.
pub fn is_same_file(source: &Path, target: &Path) -> bool {
    if source == target {
        return true;
    }
    // Canonicalizing resolves symlinks and case-insensitive file systems, but
    // fails for a target that does not exist yet, which cannot be the source.
    match (source.canonicalize(), target.canonicalize()) {
        (Ok(source), Ok(target)) => source == target,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("imechen_gpui_output_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn target_path_replaces_the_extension_inside_the_output_dir() {
        assert_eq!(
            target_path(Path::new("/in/photo.final.jpeg"), Path::new("/out"), "png"),
            PathBuf::from("/out/photo.final.png")
        );
    }

    #[test]
    fn converting_in_place_to_the_same_extension_is_the_same_file() {
        let source = Path::new("/in/photo.png");
        let target = target_path(source, Path::new("/in"), "png");
        assert!(is_same_file(source, &target));
    }

    #[test]
    fn a_different_extension_in_the_same_dir_is_not_the_same_file() {
        let source = Path::new("/in/photo.png");
        let target = target_path(source, Path::new("/in"), "webp");
        assert!(!is_same_file(source, &target));
    }

    #[test]
    fn a_dot_segment_in_the_output_dir_still_resolves_to_the_source() {
        let dir = TempDir::new("dot");
        let source = dir.0.join("a.png");
        std::fs::write(&source, b"x").unwrap();
        let target = target_path(&source, &dir.0.join("."), "png");
        assert!(is_same_file(&source, &target));
    }

    #[test]
    fn an_existing_other_file_is_not_the_same_file() {
        let dir = TempDir::new("other");
        let source = dir.0.join("a.png");
        let other = dir.0.join("b.png");
        std::fs::write(&source, b"x").unwrap();
        std::fs::write(&other, b"y").unwrap();
        assert!(!is_same_file(&source, &other));
    }
}
