use imechen_core::animation::{CreateAnimationRequest, CreateAnimationResult};

#[tauri::command]
pub async fn create_animation(
    request: CreateAnimationRequest,
) -> Result<CreateAnimationResult, String> {
    tauri::async_runtime::spawn_blocking(move || imechen_core::animation::create_animation(request))
        .await
        .map_err(|e| e.to_string())?
}
