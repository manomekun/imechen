use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const MAX_ATTEMPTS: u32 = 10_000;

/// An output path claimed by creating an empty file, removed on drop unless
/// `commit` is called so a failed conversion leaves nothing behind.
pub(crate) struct OutputReservation {
    path: PathBuf,
    committed: bool,
}

impl OutputReservation {
    /// Claims `{stem}.{ext}`, then `{stem}(1).{ext}`, `{stem}(2).{ext}`, ...
    /// in `dir`, whichever is free first.
    ///
    /// `create_new` makes the existence check and the claim a single atomic
    /// step; checking first and writing later would let a concurrent
    /// conversion or another process take the same name in between.
    pub(crate) fn reserve(dir: &Path, stem: &str, ext: &str) -> Result<Self, String> {
        for n in 0..=MAX_ATTEMPTS {
            let name = if n == 0 {
                format!("{}.{}", stem, ext)
            } else {
                format!("{}({}).{}", stem, n, ext)
            };
            let path = dir.join(name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => {
                    return Ok(Self {
                        path,
                        committed: false,
                    })
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err(format!(
            "No free output name for {}.{} in {}",
            stem,
            ext,
            dir.display()
        ))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn commit(mut self) -> PathBuf {
        self.committed = true;
        std::mem::take(&mut self.path)
    }
}

impl Drop for OutputReservation {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("imechen_output_{}_{}", tag, std::process::id()));
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

    fn reserve_name(dir: &TempDir) -> String {
        let r = OutputReservation::reserve(&dir.0, "hoge", "png").unwrap();
        r.commit()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn free_name_is_used_as_is() {
        let dir = TempDir::new("free");
        assert_eq!(reserve_name(&dir), "hoge.png");
    }

    #[test]
    fn existing_file_gets_a_number_without_space() {
        let dir = TempDir::new("one");
        std::fs::write(dir.0.join("hoge.png"), b"x").unwrap();
        assert_eq!(reserve_name(&dir), "hoge(1).png");
    }

    #[test]
    fn numbering_skips_numbers_already_taken() {
        let dir = TempDir::new("two");
        std::fs::write(dir.0.join("hoge.png"), b"x").unwrap();
        std::fs::write(dir.0.join("hoge(1).png"), b"x").unwrap();
        assert_eq!(reserve_name(&dir), "hoge(2).png");
    }

    #[test]
    fn dotted_stem_keeps_its_dots() {
        let dir = TempDir::new("dots");
        std::fs::write(dir.0.join("my.photo.webp"), b"x").unwrap();
        let r = OutputReservation::reserve(&dir.0, "my.photo", "webp").unwrap();
        assert_eq!(r.path().file_name().unwrap(), "my.photo(1).webp");
    }

    #[test]
    fn live_reservation_blocks_the_name_and_drop_releases_it() {
        let dir = TempDir::new("live");
        let first = OutputReservation::reserve(&dir.0, "hoge", "png").unwrap();
        let second = OutputReservation::reserve(&dir.0, "hoge", "png").unwrap();
        assert_eq!(first.path().file_name().unwrap(), "hoge.png");
        assert_eq!(second.path().file_name().unwrap(), "hoge(1).png");

        drop(first);

        assert!(!dir.0.join("hoge.png").exists());
        assert!(dir.0.join("hoge(1).png").exists());
    }

    #[test]
    fn missing_directory_is_an_error() {
        let dir = TempDir::new("missing");
        assert!(OutputReservation::reserve(&dir.0.join("nope"), "hoge", "png").is_err());
    }
}
