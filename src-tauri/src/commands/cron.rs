use tauri::State;

use crate::{
    db,
    models::cron::{CronJobRecord, CronRunRecord},
    services,
    state::AppState,
};

#[tauri::command]
pub fn list_cron_jobs(state: State<'_, AppState>) -> Result<Vec<CronJobRecord>, String> {
    db::list_cron_jobs(&state.db_path())
}

#[tauri::command]
pub fn list_cron_runs(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Vec<CronRunRecord>, String> {
    db::list_cron_runs(&state.db_path(), &job_id)
}

#[tauri::command]
pub fn create_cron_job(
    state: State<'_, AppState>,
    name: String,
    schedule: String,
    prompt: String,
    enabled: bool,
) -> Result<CronJobRecord, String> {
    db::create_cron_job(&state.db_path(), &name, &schedule, &prompt, enabled)
}

#[tauri::command]
pub fn update_cron_job(
    state: State<'_, AppState>,
    job_id: String,
    name: String,
    schedule: String,
    prompt: String,
    enabled: bool,
) -> Result<CronJobRecord, String> {
    db::update_cron_job(
        &state.db_path(),
        &job_id,
        &name,
        &schedule,
        &prompt,
        enabled,
    )
}

#[tauri::command]
pub fn delete_cron_job(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    db::delete_cron_job(&state.db_path(), &job_id)
}

#[tauri::command]
pub async fn run_cron_job_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<CronRunRecord, String> {
    services::cron::run_job_now(app, state.inner().clone(), job_id).await
}
