use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: String,
    pub arguments_json: String,
    pub url: String,
    pub headers_json: String,
    pub environment_json: String,
    pub enabled: bool,
    pub last_tested_at: Option<String>,
    pub last_test_status: Option<String>,
    pub last_test_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDraft {
    pub name: String,
    pub transport: String,
    pub command: String,
    pub arguments_json: String,
    pub url: String,
    pub headers_json: String,
    pub environment_json: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerTestReport {
    pub ok: bool,
    pub transport: String,
    pub message: String,
    pub details: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerTestResult {
    pub server: McpServerRecord,
    pub report: McpServerTestReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolRecord {
    pub full_name: String,
    pub tool_name: String,
    pub server_name: String,
    pub description: String,
    pub input_schema_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerToolsResult {
    pub server: McpServerRecord,
    pub tools: Vec<McpToolRecord>,
    pub discovered_at: String,
}
