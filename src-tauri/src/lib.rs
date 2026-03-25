mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::image::get_image_info,
            commands::image::convert_image,
            commands::video::get_video_info,
            commands::video::convert_video,
            commands::video::ensure_ffmpeg,
            commands::animation::create_animation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
