use tauri::State;

use crate::{
    models::mcp::{McpServerDraft, McpServerRecord, McpServerTestResult, McpServerToolsResult},
    services,
    state::AppState,
};

#[tauri::command]
pub fn list_mcp_servers(state: State<'_, AppState>) -> Result<Vec<McpServerRecord>, String> {
    services::mcp::list_servers(state.inner())
}

#[tauri::command]
pub fn create_mcp_server(
    state: State<'_, AppState>,
    server: McpServerDraft,
) -> Result<McpServerRecord, String> {
    services::mcp::create_server(state.inner(), server)
}

#[tauri::command]
pub fn update_mcp_server(
    state: State<'_, AppState>,
    server_id: String,
    server: McpServerDraft,
) -> Result<McpServerRecord, String> {
    services::mcp::update_server(state.inner(), &server_id, server)
}

#[tauri::command]
pub fn delete_mcp_server(state: State<'_, AppState>, server_id: String) -> Result<(), String> {
    services::mcp::delete_server(state.inner(), &server_id)
}

#[tauri::command]
pub fn test_mcp_server(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<McpServerTestResult, String> {
    services::mcp::test_server(state.inner(), &server_id)
}

#[tauri::command]
pub async fn discover_mcp_server_tools(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<McpServerToolsResult, String> {
    services::mcp::discover_server_tools(state.inner(), &server_id).await
}
