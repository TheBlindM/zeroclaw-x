use std::{fs, path::PathBuf};

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    db,
    models::knowledge::{KnowledgeDocumentRecord, SessionKnowledgeScopeRecord},
    state::AppState,
};

#[tauri::command]
pub fn list_project_knowledge(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<KnowledgeDocumentRecord>, String> {
    db::list_project_knowledge(&state.db_path(), &project_id)
}

#[tauri::command]
pub fn get_session_knowledge_scope(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<SessionKnowledgeScopeRecord, String> {
    db::get_session_knowledge_scope(&state.db_path(), &session_id)
}

#[tauri::command]
pub fn save_session_knowledge_scope(
    state: State<'_, AppState>,
    session_id: String,
    mode: String,
    document_ids: Vec<String>,
) -> Result<SessionKnowledgeScopeRecord, String> {
    db::save_session_knowledge_scope(&state.db_path(), &session_id, &mode, &document_ids)
}

#[tauri::command]
pub fn create_project_knowledge_note(
    state: State<'_, AppState>,
    project_id: String,
    title: String,
    content: String,
) -> Result<KnowledgeDocumentRecord, String> {
    db::create_project_knowledge_note(&state.db_path(), &project_id, &title, &content)
}

#[tauri::command]
pub async fn import_project_knowledge_files(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<KnowledgeDocumentRecord>, String> {
    let selected = app
        .dialog()
        .file()
        .add_filter(
            "Text and Markdown",
            &[
                "txt", "md", "markdown", "json", "toml", "yaml", "yml", "rs", "ts", "tsx", "js",
                "jsx", "vue", "py",
            ],
        )
        .set_title("Import project knowledge files")
        .blocking_pick_files();

    let Some(selected) = selected else {
        return Ok(Vec::new());
    };

    let paths = selected
        .into_iter()
        .map(|entry| {
            entry
                .into_path()
                .map_err(|_| "Failed to resolve one of the selected file paths.".to_string())
        })
        .collect::<Result<Vec<PathBuf>, _>>()?;

    let mut imported = Vec::new();
    for path in paths {
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
        let title = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Imported knowledge")
            .to_string();
        imported.push(db::create_project_knowledge_document(
            &state.db_path(),
            &project_id,
            &title,
            &path.display().to_string(),
            &content,
        )?);
    }

    Ok(imported)
}

#[tauri::command]
pub fn delete_knowledge_document(
    state: State<'_, AppState>,
    document_id: String,
) -> Result<(), String> {
    db::delete_knowledge_document(&state.db_path(), &document_id)
}
