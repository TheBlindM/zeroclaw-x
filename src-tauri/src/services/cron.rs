use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

use crate::{
    db,
    models::cron::{CronJobRecord, CronRunRecord},
    state::AppState,
};

const CRON_POLL_INTERVAL_SECS: u64 = 15;

pub fn start_scheduler(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(error) = tick_scheduler(&app, &state).await {
                eprintln!("cron scheduler tick failed: {error}");
            }

            sleep(Duration::from_secs(CRON_POLL_INTERVAL_SECS)).await;
        }
    });
}

pub async fn run_job_now(
    app: AppHandle,
    state: AppState,
    job_id: String,
) -> Result<CronRunRecord, String> {
    let job = db::get_cron_job(&state.db_path(), &job_id)?
        .ok_or_else(|| "Cron job not found.".to_string())?;

    execute_job(app, state, job).await
}

async fn tick_scheduler(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let due_jobs = db::list_due_cron_jobs(&state.db_path(), Utc::now().timestamp_millis())?;

    for job in due_jobs {
        if !state.try_start_cron_job(&job.id) {
            continue;
        }

        let app_handle = app.clone();
        let state_clone = state.clone();
        tauri::async_runtime::spawn(async move {
            let job_id = job.id.clone();
            if let Err(error) = execute_job_without_guard(app_handle, state_clone, job).await {
                eprintln!("cron job {job_id} failed: {error}");
            }
        });
    }

    Ok(())
}

async fn execute_job(
    app: AppHandle,
    state: AppState,
    job: CronJobRecord,
) -> Result<CronRunRecord, String> {
    if !state.try_start_cron_job(&job.id) {
        return Err("Cron job is already running.".to_string());
    }

    execute_job_without_guard(app, state, job).await
}

async fn execute_job_without_guard(
    app: AppHandle,
    state: AppState,
    job: CronJobRecord,
) -> Result<CronRunRecord, String> {
    let job_id = job.id.clone();
    let result = execute_job_inner(&app, &state, job).await;
    state.finish_cron_job(&job_id);
    result
}

async fn execute_job_inner(
    app: &AppHandle,
    state: &AppState,
    job: CronJobRecord,
) -> Result<CronRunRecord, String> {
    let db_path = state.db_path();
    let settings_path = state.settings_path();
    let started_at = Utc::now().timestamp_millis().to_string();

    let runtime_result = async {
        let runtime = super::runtime::build_runtime_session(&db_path, &settings_path)?;
        runtime
            .provider
            .simple_chat(&job.prompt, &runtime.model, runtime.temperature)
            .await
            .map_err(super::runtime::sanitize_runtime_error)
    }
    .await;

    let finished_at = Utc::now().timestamp_millis().to_string();
    let (status, output) = match runtime_result {
        Ok(output) => ("success", output),
        Err(error) => ("error", error),
    };

    let run = db::record_cron_job_run(
        &db_path,
        &job.id,
        &job.schedule,
        job.enabled,
        status,
        &output,
        &started_at,
        &finished_at,
    )?;

    emit_cron_updates(app, &db_path, &job.id, &run)?;
    Ok(run)
}

fn emit_cron_updates(
    app: &AppHandle,
    db_path: &std::path::Path,
    job_id: &str,
    run: &CronRunRecord,
) -> Result<(), String> {
    if let Some(job) = db::get_cron_job(db_path, job_id)? {
        app.emit("cron:job-updated", &job)
            .map_err(|error| error.to_string())?;
    }

    app.emit("cron:run-recorded", run)
        .map_err(|error| error.to_string())?;

    Ok(())
}
