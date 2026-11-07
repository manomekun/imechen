// Release builds would otherwise open a console window behind the GUI on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod components;
mod media;
mod output;
mod pages;
mod settings;
mod sidebar;
mod theme;

use gpui::{
    px, size, AppContext as _, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions,
};
use gpui_kit::assets::{icon_assets, Assets};

// The default bundle lacks these icons, and `Assets` errors instead of
// returning `None` for unknown paths, so the extras must be tried first.
icon_assets!(ExtraIcons, [Image, Video, Layers, Monitor, Trash, Images]);

struct AppAssets;

impl gpui::AssetSource for AppAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match ExtraIcons.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Assets.list(path)
    }
}

fn main() {
    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        // GPUI keeps a macOS app alive with no window by default, and imechen has no
        // way to reopen one, so the app would linger until quit from the Dock.
        cx.set_quit_mode(QuitMode::LastWindowClosed);

        let mode = settings::load_theme_mode();
        cx.set_global(settings::ActiveThemeMode(mode));
        theme::apply(mode, cx.window_appearance(), cx);

        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(960.), px(600.))),
            titlebar: Some(TitlebarOptions {
                title: Some("imechen".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            // The OS appearance can differ from the one read before the window existed.
            theme::apply(mode, window.appearance(), cx);
            cx.new(|cx| app::Imechen::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
