use tauri::State;

use crate::{
    models::auth::{AuthLoginChallengeRecord, AuthLoginStatusRecord, AuthProfilesStateRecord},
    services,
    state::AppState,
};

#[tauri::command]
pub async fn list_auth_profiles(
    state: State<'_, AppState>,
    provider: String,
) -> Result<AuthProfilesStateRecord, String> {
    services::auth::list_auth_profiles(&state.db_path(), &provider).await
}

#[tauri::command]
pub async fn start_auth_login(
    state: State<'_, AppState>,
    provider: String,
    profile_name: String,
) -> Result<AuthLoginChallengeRecord, String> {
    services::auth::start_auth_login(&state.db_path(), &provider, &profile_name).await
}

#[tauri::command]
pub fn get_auth_login_status(login_id: String) -> Result<AuthLoginStatusRecord, String> {
    services::auth::get_auth_login_status(&login_id)
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    services::auth::open_external_url(&url)
}
