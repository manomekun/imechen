mod app;
mod media;
mod theme;

use gpui::{px, size, AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions};

fn main() {
    // Paths on the command line are loaded as if dropped, so the UI can be
    // exercised without a native file dialog (smoke runs, screenshots).
    let initial_paths: Vec<_> = std::env::args_os().skip(1).map(Into::into).collect();

    gpui_platform::application().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("imechen (GPUI prototype)".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let window = cx
            .open_window(options, |window, cx| {
                cx.new(|cx| app::Imechen::new(window, cx))
            })
            .expect("failed to open window");
        if !initial_paths.is_empty() {
            window
                .update(cx, |view, _, cx| view.add_paths(initial_paths, cx))
                .expect("window closed before initial paths were added");
        }
        cx.activate(true);
    });
}
