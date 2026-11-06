use imechen_core::video::{ConvertVideoRequest, ConvertVideoResult, VideoInfo};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn get_video_info(path: String) -> Result<VideoInfo, String> {
    tauri::async_runtime::spawn_blocking(move || imechen_core::video::get_video_info(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn convert_video(
    app: AppHandle,
    request: ConvertVideoRequest,
) -> Result<ConvertVideoResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        imechen_core::video::convert_video(request, |progress| {
            let _ = app.emit("video-progress", progress);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ensure_ffmpeg() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(imechen_core::video::ensure_ffmpeg)
        .await
        .map_err(|e| e.to_string())?
}
