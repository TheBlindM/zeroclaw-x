use tauri::{AppHandle, State};

use crate::{
    models::skill::{SkillDetailRecord, SkillDraft, SkillRecord, SkillTemplateRecord},
    services,
    state::AppState,
};

#[tauri::command]
pub fn list_skill_templates() -> Result<Vec<SkillTemplateRecord>, String> {
    Ok(services::skill::list_templates())
}

#[tauri::command]
pub fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillRecord>, String> {
    services::skill::list_skills(state.inner())
}

#[tauri::command]
pub fn create_skill(state: State<'_, AppState>, skill: SkillDraft) -> Result<SkillRecord, String> {
    services::skill::create_skill(state.inner(), &skill)
}

#[tauri::command]
pub fn get_skill_detail(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<SkillDetailRecord, String> {
    services::skill::get_skill_detail(state.inner(), &skill_id)
}

#[tauri::command]
pub fn install_skill_template(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<SkillRecord, String> {
    services::skill::install_template(state.inner(), &template_id)
}

#[tauri::command]
pub fn import_skill_directory(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<SkillRecord>, String> {
    services::skill::import_skill_directory(&app, state.inner())
}

#[tauri::command]
pub fn set_skill_enabled(
    state: State<'_, AppState>,
    skill_id: String,
    enabled: bool,
) -> Result<SkillRecord, String> {
    services::skill::set_skill_enabled(state.inner(), &skill_id, enabled)
}

#[tauri::command]
pub fn delete_skill(state: State<'_, AppState>, skill_id: String) -> Result<SkillRecord, String> {
    services::skill::delete_skill(state.inner(), &skill_id)
}
