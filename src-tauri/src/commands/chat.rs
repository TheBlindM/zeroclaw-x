use tauri::{Emitter, State};

use crate::{
    db,
    models::chat::{ChatApprovalDecision, ChatErrorPayload, MessageRecord, SessionRecord},
    services,
    state::AppState,
};

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    session_title: Option<String>,
    project_id: Option<String>,
    knowledge_mode: Option<String>,
    knowledge_document_ids: Option<Vec<String>>,
    agent_mode: Option<bool>,
) -> Result<(), String> {
    let title = session_title
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| content.chars().take(48).collect::<String>());
    let db_path = state.db_path();
    let run_agent_mode = agent_mode.unwrap_or(false);

    db::upsert_session(&db_path, &session_id, &title)?;
    db::set_session_agent_mode(&db_path, &session_id, run_agent_mode)?;
    db::assign_session_project(&db_path, &session_id, project_id.as_deref())?;
    db::save_session_knowledge_scope(
        &db_path,
        &session_id,
        knowledge_mode.as_deref().unwrap_or("auto"),
        &knowledge_document_ids.unwrap_or_default(),
    )?;
    db::record_message(&db_path, &session_id, "user", &content)?;
    state.clear_cancellation(&session_id);

    let app_handle = app.clone();
    let state_clone = state.inner().clone();

    tauri::async_runtime::spawn(async move {
        if let Err(error) = services::chat::stream_response(
            app_handle.clone(),
            state_clone,
            session_id.clone(),
            content,
            run_agent_mode,
        )
        .await
        {
            let _ = app_handle.emit("chat:error", ChatErrorPayload { session_id, error });
        }
    });

    Ok(())
}

#[tauri::command]
pub fn respond_to_tool_approval(
    state: State<'_, AppState>,
    request_id: String,
    decision: ChatApprovalDecision,
) -> Result<(), String> {
    state.resolve_approval(&request_id, decision)
}

#[tauri::command]
pub fn stop_message(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    state.cancel_session(&session_id);
    Ok(())
}

#[tauri::command]
pub fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionRecord>, String> {
    db::list_sessions(&state.db_path())
}

#[tauri::command]
pub fn list_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<MessageRecord>, String> {
    db::list_messages(&state.db_path(), &session_id)
}

#[tauri::command]
pub fn rename_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    db::rename_session(&state.db_path(), &session_id, title.trim())
}

#[tauri::command]
pub fn set_session_agent_mode(
    state: State<'_, AppState>,
    session_id: String,
    agent_mode: bool,
) -> Result<(), String> {
    db::set_session_agent_mode(&state.db_path(), &session_id, agent_mode)
}

#[tauri::command]
pub fn assign_session_project(
    state: State<'_, AppState>,
    session_id: String,
    project_id: Option<String>,
) -> Result<(), String> {
    db::assign_session_project(&state.db_path(), &session_id, project_id.as_deref())
}

#[tauri::command]
pub fn delete_session(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    db::delete_session(&state.db_path(), &session_id)?;
    state.clear_session_runtime_state(&session_id);
    Ok(())
}
