use tauri::{AppHandle, State};

use crate::{
    models::update::{UpdateCheckReport, UpdateInstallReport, UpdateSettingsRecord},
    services,
    state::AppState,
};

#[tauri::command]
pub fn get_update_settings(state: State<'_, AppState>) -> Result<UpdateSettingsRecord, String> {
    services::update::load_update_settings(&state.update_settings_path())
}

#[tauri::command]
pub fn save_update_settings(
    state: State<'_, AppState>,
    settings: UpdateSettingsRecord,
) -> Result<UpdateSettingsRecord, String> {
    services::update::save_update_settings(&state.update_settings_path(), settings)
}

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateCheckReport, String> {
    services::update::check_for_update(&app, &state.update_settings_path()).await
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateInstallReport, String> {
    services::update::install_update(&app, &state.update_settings_path()).await
}
