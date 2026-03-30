use tauri::State;

use crate::{
    db,
    models::{chat::SessionRecord, project::ProjectRecord},
    state::AppState,
};

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectRecord>, String> {
    db::list_projects(&state.db_path())
}

#[tauri::command]
pub fn list_project_sessions(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<SessionRecord>, String> {
    db::list_project_sessions(&state.db_path(), &project_id)
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    name: String,
    description: String,
    status: String,
    pinned: bool,
) -> Result<ProjectRecord, String> {
    db::create_project(&state.db_path(), &name, &description, &status, pinned)
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    description: String,
    status: String,
    pinned: bool,
) -> Result<ProjectRecord, String> {
    db::update_project(
        &state.db_path(),
        &project_id,
        &name,
        &description,
        &status,
        pinned,
    )
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, project_id: String) -> Result<(), String> {
    db::delete_project(&state.db_path(), &project_id)
}
