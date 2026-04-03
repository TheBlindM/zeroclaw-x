use tauri::{AppHandle, State};

use crate::{
    models::settings::{
        RuntimeConnectionReport, RuntimeProfileRecord, RuntimeProfilesExportReport,
        RuntimeProfilesImportReport, RuntimeProfilesState, RuntimeProxySettingsRecord,
        RuntimeProxySupportRecord, RuntimeSettingsRecord, RuntimeStatusRecord,
    },
    services,
    state::AppState,
};

#[tauri::command]
pub fn get_runtime_profiles(state: State<'_, AppState>) -> Result<RuntimeProfilesState, String> {
    services::runtime::load_runtime_profiles(&state.settings_path())
}

#[tauri::command]
pub fn get_runtime_settings(state: State<'_, AppState>) -> Result<RuntimeSettingsRecord, String> {
    services::runtime::load_runtime_settings(&state.settings_path())
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatusRecord, String> {
    services::runtime::get_runtime_status(&state.db_path(), &state.settings_path())
}

#[tauri::command]
pub fn get_proxy_settings(
    state: State<'_, AppState>,
) -> Result<RuntimeProxySettingsRecord, String> {
    services::runtime::load_proxy_settings(&state.settings_path())
}

#[tauri::command]
pub fn save_proxy_settings(
    state: State<'_, AppState>,
    settings: RuntimeProxySettingsRecord,
) -> Result<RuntimeProxySettingsRecord, String> {
    services::runtime::save_proxy_settings(&state.settings_path(), settings)
}

#[tauri::command]
pub fn get_proxy_support(_: State<'_, AppState>) -> Result<RuntimeProxySupportRecord, String> {
    Ok(services::runtime::get_proxy_support())
}

#[tauri::command]
pub fn save_runtime_settings(
    state: State<'_, AppState>,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeProfilesState, String> {
    let profiles = services::runtime::save_runtime_settings(&state.settings_path(), settings)?;
    sync_runtime_skills(state.inner())?;
    Ok(profiles)
}

#[tauri::command]
pub fn create_runtime_profile(
    state: State<'_, AppState>,
    name: String,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeProfilesState, String> {
    let profiles =
        services::runtime::create_runtime_profile(&state.settings_path(), &name, settings)?;
    sync_runtime_skills(state.inner())?;
    Ok(profiles)
}

#[tauri::command]
pub fn update_runtime_profile(
    state: State<'_, AppState>,
    profile_id: String,
    name: String,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeProfilesState, String> {
    let profiles = services::runtime::update_runtime_profile(
        &state.settings_path(),
        &profile_id,
        &name,
        settings,
    )?;
    sync_runtime_skills(state.inner())?;
    Ok(profiles)
}

#[tauri::command]
pub fn activate_runtime_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<RuntimeProfilesState, String> {
    let profiles =
        services::runtime::activate_runtime_profile(&state.settings_path(), &profile_id)?;
    sync_runtime_skills(state.inner())?;
    Ok(profiles)
}

#[tauri::command]
pub fn delete_runtime_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<RuntimeProfilesState, String> {
    let profiles = services::runtime::delete_runtime_profile(&state.settings_path(), &profile_id)?;
    sync_runtime_skills(state.inner())?;
    Ok(profiles)
}

#[tauri::command]
pub async fn export_runtime_profiles(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<RuntimeProfilesExportReport>, String> {
    services::runtime::export_runtime_profiles(&app, &state.settings_path())
}

#[tauri::command]
pub async fn import_runtime_profiles(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<RuntimeProfilesImportReport>, String> {
    let report = services::runtime::import_runtime_profiles(&app, &state.settings_path())?;
    if report.is_some() {
        sync_runtime_skills(state.inner())?;
    }
    Ok(report)
}

#[tauri::command]
pub async fn pick_runtime_workspace(app: AppHandle) -> Result<Option<String>, String> {
    services::runtime::pick_runtime_workspace(&app)
}

#[tauri::command]
pub async fn test_runtime_settings(
    state: State<'_, AppState>,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeConnectionReport, String> {
    services::runtime::test_runtime_settings(&state.db_path(), settings).await
}

#[tauri::command]
pub async fn test_runtime_profile(
    state: State<'_, AppState>,
    profile: RuntimeProfileRecord,
) -> Result<RuntimeConnectionReport, String> {
    services::runtime::test_runtime_settings(&state.db_path(), profile.settings).await
}

fn sync_runtime_skills(state: &AppState) -> Result<(), String> {
    services::skill::sync_runtime_skills(&state.db_path(), &state.settings_path())
}
