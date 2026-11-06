use std::fs;
use std::path::PathBuf;

use gpui::{App, Global};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Dark,
    Light,
    #[default]
    System,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    /// Unknown content falls back to `System` so a corrupted file never blocks startup.
    pub fn parse(text: &str) -> Self {
        match text.trim() {
            "dark" => Self::Dark,
            "light" => Self::Light,
            _ => Self::System,
        }
    }
}

/// Kept as a global so the settings page and the appearance observer in the
/// root view agree on the mode without wiring entities together.
pub struct ActiveThemeMode(pub ThemeMode);

impl Global for ActiveThemeMode {}

pub fn theme_mode(cx: &App) -> ThemeMode {
    cx.try_global::<ActiveThemeMode>()
        .map(|mode| mode.0)
        .unwrap_or_default()
}

fn theme_path(config_dir: Option<PathBuf>) -> Option<PathBuf> {
    config_dir.map(|dir| dir.join("imechen").join("theme"))
}

pub fn load_theme_mode() -> ThemeMode {
    theme_path(dirs::config_dir())
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|text| ThemeMode::parse(&text))
        .unwrap_or_default()
}

pub fn save_theme_mode(mode: ThemeMode) {
    let Some(path) = theme_path(dirs::config_dir()) else {
        return;
    };
    let result = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&path, mode.as_str()));
    if let Err(e) = result {
        eprintln!("failed to save theme mode to {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ThemeMode; 3] = [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System];

    #[test]
    fn every_mode_survives_a_serialize_parse_round_trip() {
        for mode in ALL {
            assert_eq!(ThemeMode::parse(mode.as_str()), mode);
        }
    }

    #[test]
    fn invalid_or_empty_content_falls_back_to_system() {
        for text in ["", "Dark", "purple", "dark light", "{}"] {
            assert_eq!(ThemeMode::parse(text), ThemeMode::System, "{text:?}");
        }
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert_eq!(ThemeMode::parse("dark\n"), ThemeMode::Dark);
        assert_eq!(ThemeMode::parse("  light "), ThemeMode::Light);
    }

    #[test]
    fn default_mode_is_system() {
        assert_eq!(ThemeMode::default(), ThemeMode::System);
    }

    #[test]
    fn theme_file_lives_under_an_imechen_directory_in_the_config_dir() {
        assert_eq!(
            theme_path(Some(PathBuf::from("/cfg"))),
            Some(PathBuf::from("/cfg/imechen/theme"))
        );
    }

    #[test]
    fn theme_file_has_no_path_without_a_config_dir() {
        assert_eq!(theme_path(None), None);
    }
}
