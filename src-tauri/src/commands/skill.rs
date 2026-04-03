use tauri::{AppHandle, State};

use crate::{
    models::skill::{
        SkillAssetImportReport, SkillDetailRecord, SkillDraft, SkillEntryDraft, SkillExportReport,
        SkillFileContentRecord, SkillRecord, SkillTemplateRecord,
    },
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
pub fn update_skill(
    state: State<'_, AppState>,
    skill_id: String,
    skill: SkillDraft,
) -> Result<SkillRecord, String> {
    services::skill::update_skill(state.inner(), &skill_id, &skill)
}

#[tauri::command]
pub fn get_skill_detail(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<SkillDetailRecord, String> {
    services::skill::get_skill_detail(state.inner(), &skill_id)
}

#[tauri::command]
pub fn get_skill_file_content(
    state: State<'_, AppState>,
    skill_id: String,
    relative_path: String,
) -> Result<SkillFileContentRecord, String> {
    services::skill::get_skill_file_content(state.inner(), &skill_id, &relative_path)
}

#[tauri::command]
pub fn save_skill_file_content(
    state: State<'_, AppState>,
    skill_id: String,
    relative_path: String,
    content: String,
) -> Result<SkillFileContentRecord, String> {
    services::skill::save_skill_file_content(state.inner(), &skill_id, &relative_path, &content)
}

#[tauri::command]
pub fn create_skill_entry(
    state: State<'_, AppState>,
    skill_id: String,
    draft: SkillEntryDraft,
) -> Result<SkillDetailRecord, String> {
    services::skill::create_skill_entry(state.inner(), &skill_id, &draft)
}

#[tauri::command]
pub fn delete_skill_entry(
    state: State<'_, AppState>,
    skill_id: String,
    relative_path: String,
) -> Result<SkillDetailRecord, String> {
    services::skill::delete_skill_entry(state.inner(), &skill_id, &relative_path)
}

#[tauri::command]
pub fn install_skill_template(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<SkillRecord, String> {
    services::skill::install_template(state.inner(), &template_id)
}

#[tauri::command]
pub async fn import_skill_directory(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<SkillRecord>, String> {
    services::skill::import_skill_directory(&app, state.inner())
}

#[tauri::command]
pub fn duplicate_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<SkillRecord, String> {
    services::skill::duplicate_skill(state.inner(), &skill_id)
}

#[tauri::command]
pub fn refresh_skill(state: State<'_, AppState>, skill_id: String) -> Result<SkillRecord, String> {
    services::skill::refresh_skill(state.inner(), &skill_id)
}

#[tauri::command]
pub async fn export_skill(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Option<SkillExportReport>, String> {
    services::skill::export_skill(&app, state.inner(), &skill_id)
}

#[tauri::command]
pub async fn import_skill_assets(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Option<SkillAssetImportReport>, String> {
    services::skill::import_skill_assets(&app, state.inner(), &skill_id)
}

#[tauri::command]
pub fn open_skill_directory(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<String, String> {
    services::skill::open_skill_directory(state.inner(), &skill_id)
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
