use tauri::{AppHandle, State};

use crate::{
    models::channel::{ChannelDraft, ChannelRecord, ChannelRuntimeStatusRecord, ChannelTestResult},
    services,
    state::AppState,
};

#[tauri::command]
pub fn list_channels(state: State<'_, AppState>) -> Result<Vec<ChannelRecord>, String> {
    services::channel::list_channels(state.inner())
}

#[tauri::command]
pub fn create_channel(
    state: State<'_, AppState>,
    channel: ChannelDraft,
) -> Result<ChannelRecord, String> {
    services::channel::create_channel(state.inner(), channel)
}

#[tauri::command]
pub fn update_channel(
    state: State<'_, AppState>,
    channel_id: String,
    channel: ChannelDraft,
) -> Result<ChannelRecord, String> {
    services::channel::update_channel(state.inner(), &channel_id, channel)
}

#[tauri::command]
pub fn delete_channel(state: State<'_, AppState>, channel_id: String) -> Result<(), String> {
    services::channel::delete_channel(state.inner(), &channel_id)
}

#[tauri::command]
pub async fn test_channel(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<ChannelTestResult, String> {
    services::channel::test_channel(state.inner(), &channel_id).await
}

#[tauri::command]
pub fn get_channel_runtime_status(
    state: State<'_, AppState>,
) -> Result<ChannelRuntimeStatusRecord, String> {
    Ok(services::channel::get_runtime_status(state.inner()))
}

#[tauri::command]
pub async fn start_channel_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ChannelRuntimeStatusRecord, String> {
    services::channel::start_runtime(app, state.inner().clone()).await
}

#[tauri::command]
pub fn stop_channel_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ChannelRuntimeStatusRecord, String> {
    services::channel::stop_runtime(app, state.inner())
}
