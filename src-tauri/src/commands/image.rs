use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use imechen_core::image::{ConvertImageRequest, ConvertImageResult};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ImageInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub thumbnail: String,
}

#[tauri::command]
pub async fn get_image_info(path: String) -> Result<ImageInfo, String> {
    let info = tauri::async_runtime::spawn_blocking(move || imechen_core::image::get_image_info(&path))
        .await
        .map_err(|e| e.to_string())??;

    Ok(ImageInfo {
        path: info.path,
        name: info.name,
        size: info.size,
        width: info.width,
        height: info.height,
        format: info.format,
        thumbnail: format!("data:image/png;base64,{}", STANDARD.encode(&info.thumbnail_png)),
    })
}

#[tauri::command]
pub async fn convert_image(request: ConvertImageRequest) -> Result<ConvertImageResult, String> {
    tauri::async_runtime::spawn_blocking(move || imechen_core::image::convert_image(request))
        .await
        .map_err(|e| e.to_string())?
}
